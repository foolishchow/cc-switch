//! `#[command_api(rest = "METHOD /path", path = "id", body = "a,b", query = "c")]`
//!
//! **单一真相源 → 双生成**:一个纯 fn,宏生成三件套:
//! 1. `<name>_impl` —— 原函数体改名(取 `&AppState`,纯函数,可单测,无需 Tauri 运行时)
//! 2. `<name>` —— `#[tauri::command]` 适配(state.inner() + AppError→String)
//! 3. (仅 `rest_api` feature)`<name>_rest` —— axum handler,按 path/query/body 取参
//!
//! 约定:名为 `state`、类型 `&AppState` 的参数注入 state;其余参数按
//! `path`/`body`/`query` 属性路由到 axum 提取器(query 默认 = 未声明 path/body 的参)。
//! 路径中 `{id}` 自动转 axum `:id`。返回 `Result<T: Serialize, AppError>`。

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    FnArg, ItemFn, LitStr, Pat, Result, Token, Type,
};

/// 属性:`rest = "POST /p", path = "id", body = "a,b", query = "c"`(均除 rest 可选)
struct CommandApiAttr {
    rest: Option<String>,
    path: Option<String>,
    body: Option<String>,
    query: Option<String>,
}

impl Parse for CommandApiAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut rest = None;
        let mut path = None;
        let mut body = None;
        let mut query = None;
        while !input.is_empty() {
            let k: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let v: LitStr = input.parse()?;
            match k.to_string().as_str() {
                "rest" => rest = Some(v.value()),
                "path" => path = Some(v.value()),
                "body" => body = Some(v.value()),
                "query" => query = Some(v.value()),
                other => {
                    return Err(syn::Error::new(k.span(), format!("未知属性 {other}")))
                }
            }
            let _ = input.parse::<Token![,]>();
        }
        Ok(Self { rest, path, body, query })
    }
}

fn split_names(s: &Option<String>) -> Vec<String> {
    s.as_deref()
        .unwrap_or("")
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

#[proc_macro_attribute]
pub fn command_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as CommandApiAttr);
    let mut func = parse_macro_input!(item as ItemFn);

    let name = func.sig.ident.clone();
    let impl_name = format_ident!("{}_impl", name);
    let is_async = func.sig.asyncness.is_some();

    // 分离 state 参数与其余参数(保留声明顺序)
    let mut state_ty: Option<Type> = None;
    let mut other_args: Vec<(syn::Ident, Type)> = Vec::new();
    for arg in &func.sig.inputs {
        if let FnArg::Typed(pt) = arg {
            let ty = &pt.ty;
            if let Pat::Ident(pi) = &*pt.pat {
                if pi.ident == "state" {
                    state_ty = Some(strip_ref(ty));
                } else {
                    other_args.push((pi.ident.clone(), (**ty).clone()));
                }
            }
        }
    }
    let state_ty = match state_ty {
        Some(t) => t,
        None => {
            return syn::Error::new(name.span(), "command_api 需要 `state: &AppState` 参数")
                .to_compile_error()
                .into();
        }
    };

    let ok_ty = extract_ok_type(&func.sig.output);

    // —— 1. impl(改名,原函数体)——
    func.sig.ident = impl_name.clone();
    let attrs = &func.attrs;
    let vis = &func.vis;
    let sig = &func.sig;
    let block = &func.block;
    let impl_fn = quote! {
        #(#attrs)*
        #vis #sig #block
    };

    // —— 2. Tauri command 适配(Tauri 端不需路由,所有参从 invoke args 取)——
    let other_pats: Vec<_> = other_args.iter().map(|(i, _)| i.clone()).collect();
    let other_inputs: TokenStream2 = other_args
        .iter()
        .map(|(i, t)| quote! { #i: #t, })
        .collect();
    let await_tok = if is_async { Some(quote! {.await}) } else { None };
    let async_kw = if is_async { Some(quote! {async}) } else { None };
    let tauri_cmd = quote! {
        #[tauri::command]
        #vis #async_kw fn #name(
            state: tauri::State<'_, #state_ty>,
            #other_inputs
        ) -> std::result::Result<#ok_ty, String> {
            #impl_name(state.inner(), #(#other_pats),*) #await_tok
                .map_err(|e| e.to_string())
        }
    };

    // —— 3. axum handler(仅 rest_api feature)——
    let rest_handler = if let Some(rest_str) = &attr.rest {
        let upper = name.to_string().to_uppercase();
        let path_names = split_names(&attr.path);
        let body_names = split_names(&attr.body);
        // query 默认 = 未声明 path/body 的其余参;显式 query 覆盖
        let query_names: Vec<String> = if attr.query.is_some() {
            split_names(&attr.query)
        } else {
            other_args
                .iter()
                .map(|(i, _)| i.to_string())
                .filter(|n| !path_names.contains(n) && !body_names.contains(n))
                .collect()
        };

        // 解析 method + 路径,{id} → :id(axum 动态段)——先于 ident 计算
        let (method, axum_path) = parse_rest_route(rest_str);

        let rest_fn_name = format_ident!("{}_rest", name);
        let rest_meta_const = format_ident!("{}_REST", upper);
        let mount_fn_name = format_ident!("{}_mount", name);
        // axum routing 函数名(get/post/put/delete...)
        let method_fn = format_ident!("{}", method.to_lowercase());
        let path_struct = format_ident!("{}Path", upper);
        let query_struct = format_ident!("{}Query", upper);
        let body_struct = format_ident!("{}Body", upper);

        // 各提取器结构体字段(从 other_args 取类型)
        let path_fields = fields_for(&path_names, &other_args);
        let query_fields = fields_for(&query_names, &other_args);
        let body_fields = fields_for(&body_names, &other_args);

        // 调用 impl 的实参(按原声明顺序,从对应提取器取)
        let call_args: TokenStream2 = other_args
            .iter()
            .map(|(i, _)| {
                if path_names.contains(&i.to_string()) {
                    quote! { p.#i, }
                } else if body_names.contains(&i.to_string()) {
                    quote! { b.#i, }
                } else {
                    quote! { q.#i, }
                }
            })
            .collect();

        // 提取器参数(顺序:Path, Query, Json——body 必须最后)
        let path_extractor = if path_names.is_empty() {
            quote! {}
        } else {
            quote! { ::axum::extract::Path(p): ::axum::extract::Path<#path_struct>, }
        };
        let query_extractor = if query_names.is_empty() {
            quote! {}
        } else {
            quote! { ::axum::extract::Query(q): ::axum::extract::Query<#query_struct>, }
        };
        let body_extractor = if body_names.is_empty() {
            quote! {}
        } else {
            quote! { ::axum::Json(b): ::axum::Json<#body_struct>, }
        };

        // 结构体定义(仅有字段的才生成;需 Deserialize)
        let path_struct_def = if path_names.is_empty() {
            quote! {}
        } else {
            quote! {
                #[cfg(feature = "rest_api")]
                #[derive(::serde::Deserialize)]
                #[doc(hidden)]
                pub struct #path_struct { #path_fields }
            }
        };
        let query_struct_def = if query_names.is_empty() {
            quote! {}
        } else {
            quote! {
                #[cfg(feature = "rest_api")]
                #[derive(::serde::Deserialize)]
                #[doc(hidden)]
                pub struct #query_struct { #query_fields }
            }
        };
        let body_struct_def = if body_names.is_empty() {
            quote! {}
        } else {
            quote! {
                #[cfg(feature = "rest_api")]
                #[derive(::serde::Deserialize)]
                #[doc(hidden)]
                pub struct #body_struct { #body_fields }
            }
        };

        Some(quote! {
            /// REST 路由元信息(method + path),供注册器收集
            #[cfg(feature = "rest_api")]
            #[doc(hidden)]
            pub const #rest_meta_const: (&str, &str) = (#method, #axum_path);

            #path_struct_def
            #query_struct_def
            #body_struct_def

            /// axum handler——按 path/query/body 取参,调 `_impl`,返 JSON。
            #[cfg(feature = "rest_api")]
            #vis async fn #rest_fn_name(
                ::axum::extract::State(state): ::axum::extract::State<#state_ty>,
                #path_extractor
                #query_extractor
                #body_extractor
            ) -> std::result::Result<impl ::axum::response::IntoResponse, ::axum::http::StatusCode> {
                #impl_name(&state, #call_args) #await_tok
                    .map(::axum::Json)
                    .map_err(|_| ::axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }

            /// 路由挂载器(非捕获 fn,可作 fn 指针存入注册器)
            #[cfg(feature = "rest_api")]
            #[doc(hidden)]
            pub fn #mount_fn_name(
                r: ::axum::Router<#state_ty>,
            ) -> ::axum::Router<#state_ty> {
                r.route(#axum_path, ::axum::routing::#method_fn(#rest_fn_name))
            }

            /// 自动提交到 inventory 注册器——解耦即上路由表,无 parity drift。
            #[cfg(feature = "rest_api")]
            ::inventory::submit! {
                crate::rest_registry::RouteReg {
                    method: #method,
                    path: #axum_path,
                    mount: #mount_fn_name,
                }
            }
        })
    } else {
        None
    };

    let out = quote! {
        #impl_fn
        #tauri_cmd
        #rest_handler
    };
    out.into()
}

/// 由名字列表 + 原参类型,生成结构体字段 token(`pub name: Ty,`)
fn fields_for(names: &[String], args: &[(syn::Ident, Type)]) -> TokenStream2 {
    names
        .iter()
        .filter_map(|n| {
            args.iter()
                .find(|(i, _)| i == n)
                .map(|(i, t)| quote! { pub #i: #t, })
        })
        .collect()
}

/// `"POST /control/v1/providers/{id}"` → (`"POST"`, `"/control/v1/providers/:id"`)
fn parse_rest_route(s: &str) -> (String, String) {
    let s = s.trim();
    let (method, path) = match s.split_once(' ') {
        Some((m, p)) => (m.to_string(), p.trim().to_string()),
        None => ("GET".to_string(), s.to_string()),
    };
    // {id} → :id
    let axum_path = path.replace('{', ":").replace('}', "");
    (method, axum_path)
}

/// 剥掉 `&T` / `&mut T` 的引用层,返回 T。
fn strip_ref(ty: &Type) -> Type {
    match ty {
        Type::Reference(r) => (*r.elem).clone(),
        other => other.clone(),
    }
}

/// 从 `-> Result<T, E>` 取出 T;解析失败则回退 `()`.
fn extract_ok_type(output: &syn::ReturnType) -> Type {
    if let syn::ReturnType::Type(_, ty) = output {
        if let Type::Path(p) = &**ty {
            if let Some(seg) = p.path.segments.last() {
                if seg.ident == "Result" {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(t)) = args.args.first() {
                            return t.clone();
                        }
                    }
                }
            }
        }
    }
    syn::parse_quote!()
}
