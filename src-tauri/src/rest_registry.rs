//! REST 路由注册器（仅 `rest_api` feature）。
//!
//! `#[command_api]` 宏为每个命令 `inventory::submit!` 一个 [`RouteReg`] +
//! 一个 [`RouteMeta`]，此模块用 `inventory::iter` 收集全部 → 自动建
//! `axum::Router` + 暴露路由发现表。
//!
//! **E2 论点**:命令一解耦即自动出现在 REST 路由表，无 parity drift——
//! 凡挂 `#[command_api]` 的命令零行手写即上路由表。

#[cfg(feature = "rest_api")]
pub struct RouteReg {
    #[allow(dead_code)]
    pub method: &'static str,
    #[allow(dead_code)]
    pub path: &'static str,
    #[allow(dead_code)]
    pub mount: fn(axum::Router<crate::store::AppState>) -> axum::Router<crate::store::AppState>,
}

#[cfg(feature = "rest_api")]
inventory::collect!(RouteReg);

/// 路由元数据（供 `GET /control/v1/__routes` 发现端点 dump）。
#[cfg(feature = "rest_api")]
#[derive(serde::Serialize)]
pub struct RouteMeta {
    pub cmd: &'static str,
    pub method: &'static str,
    /// axum 路径形式（`:id` 动态段）
    pub path: &'static str,
    /// path 参名
    #[serde(rename = "pathParams")]
    pub path_params: &'static [&'static str],
}

#[cfg(feature = "rest_api")]
inventory::collect!(RouteMeta);

/// 收集所有 `#[command_api]` 提交的路由，建一个 axum Router（state = AppState）。
#[cfg(feature = "rest_api")]
#[allow(dead_code)]  // PoC：仅测试调用；正式接线后由 server setup 调用
pub fn build_router() -> axum::Router<crate::store::AppState> {
    let mut r = axum::Router::new();
    for reg in inventory::iter::<RouteReg> {
        r = (reg.mount)(r);
    }
    r
}

/// dump 全部 `#[command_api]` 路由元数据（按 cmd 排序，供发现端点）。
#[cfg(feature = "rest_api")]
#[allow(dead_code)]
pub fn route_table() -> Vec<RouteMeta> {
    let mut v: Vec<&RouteMeta> = inventory::iter::<RouteMeta>().collect();
    v.sort_by_key(|m| m.cmd);
    v.into_iter()
        .map(|m| RouteMeta {
            cmd: m.cmd,
            method: m.method,
            path: m.path,
            path_params: m.path_params,
        })
        .collect()
}

#[cfg(all(test, feature = "rest_api"))]
mod tests {
    use super::build_router;
    use crate::database::Database;
    use crate::store::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    /// E2E：inventory 自动收集路由 → build_router → oneshot 请求 → 200。
    /// 证明“解耦即上路由表”，无需手维护 route 文件。
    #[tokio::test]
    async fn rest_registry_auto_builds_and_serves_routes() {
        let db = Arc::new(Database::memory().expect("in-memory db"));
        let state = AppState::new(db);
        let app = build_router().with_state(state);

        // GET /control/v1/providers?app=claude → 200（空库 → 空列表）
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/control/v1/providers?app=claude")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // GET /control/v1/failover/queue?app_type=codex → 200（空队列）
        let res = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/control/v1/failover/queue?app_type=codex")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // 证明多种方法路径都自动注册了：至少 5 条路由
        let count = inventory::iter::<super::RouteReg>().count();
        assert!(
            count >= 5,
            "应自动收集至少 5 条路由，实际 {count}"
        );
    }
}
