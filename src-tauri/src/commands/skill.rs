//! Skills 命令层
//!
//! v3.10.0+ 统一管理架构：
//! - 支持三应用开关（Claude/Codex/Gemini）
//! - SSOT 存储在 ~/.cc-switch/skills/

use crate::app_config::{AppType, InstalledSkill, UnmanagedSkill};
use crate::error::{format_skill_error, AppError};
use crate::services::skill::{
    DiscoverableSkill, ImportSkillSelection, MigrationResult, Skill, SkillBackupEntry, SkillRepo,
    SkillService, SkillStorageLocation, SkillUninstallResult, SkillUpdateInfo,
    SkillsShSearchResult,
};
use crate::store::AppState;
use cc_command_api::command_api;
use std::str::FromStr;
use std::sync::Arc;

/// SkillService 状态包装
pub struct SkillServiceState(pub Arc<SkillService>);

/// 解析 app 参数为 AppType
fn parse_app_type(app: &str) -> Result<AppType, String> {
    AppType::from_str(app).map_err(|e| e.to_string())
}

// ========== 统一管理命令 ==========

/// 获取所有已安装的 Skills
#[command_api]
pub fn get_installed_skills(state: &AppState) -> Result<Vec<InstalledSkill>, AppError> {
    Ok(SkillService::get_all_installed(&state.db).map_err(|e| e.to_string())?)
}

#[command_api]
pub fn get_skill_backups() -> Result<Vec<SkillBackupEntry>, AppError> {
    Ok(SkillService::list_backups().map_err(|e| e.to_string())?)
}

#[command_api]
pub fn delete_skill_backup(backup_id: String) -> Result<bool, AppError> {
    SkillService::delete_backup(&backup_id).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 安装 Skill（新版统一安装）
///
/// 参数：
/// - skill: 从发现列表获取的技能信息
/// - current_app: 当前选中的应用，安装后默认启用该应用
#[command_api(state = "service, app_state")]
pub async fn install_skill_unified(
    skill: DiscoverableSkill,
    current_app: String,
    service: &SkillServiceState,
    app_state: &AppState,
) -> Result<InstalledSkill, AppError> {
    let app_type = parse_app_type(&current_app)?;
    Ok(service
        .0
        .install(&app_state.db, &skill, &app_type)
        .await
        .map_err(|e| e.to_string())?)
}

/// 卸载 Skill（新版统一卸载）
#[command_api]
pub fn uninstall_skill_unified(
    id: String,
    state: &AppState,
) -> Result<SkillUninstallResult, AppError> {
    Ok(SkillService::uninstall(&state.db, &id).map_err(|e| e.to_string())?)
}

#[command_api]
pub fn restore_skill_backup(
    backup_id: String,
    current_app: String,
    state: &AppState,
) -> Result<InstalledSkill, AppError> {
    let app_type = parse_app_type(&current_app)?;
    Ok(
        SkillService::restore_from_backup(&state.db, &backup_id, &app_type)
            .map_err(|e| e.to_string())?,
    )
}

/// 切换 Skill 的应用启用状态
#[command_api]
pub fn toggle_skill_app(
    id: String,
    app: String,
    enabled: bool,
    state: &AppState,
) -> Result<bool, AppError> {
    let app_type = parse_app_type(&app)?;
    SkillService::toggle_app(&state.db, &id, &app_type, enabled).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 扫描未管理的 Skills
#[command_api]
pub fn scan_unmanaged_skills(state: &AppState) -> Result<Vec<UnmanagedSkill>, AppError> {
    Ok(SkillService::scan_unmanaged(&state.db).map_err(|e| e.to_string())?)
}

/// 从应用目录导入 Skills
#[command_api]
pub fn import_skills_from_apps(
    imports: Vec<ImportSkillSelection>,
    state: &AppState,
) -> Result<Vec<InstalledSkill>, AppError> {
    Ok(SkillService::import_from_apps(&state.db, imports).map_err(|e| e.to_string())?)
}

// ========== 发现功能命令 ==========

/// 发现可安装的 Skills（从仓库获取）
#[command_api(state = "service, app_state")]
pub async fn discover_available_skills(
    service: &SkillServiceState,
    app_state: &AppState,
) -> Result<Vec<DiscoverableSkill>, AppError> {
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    Ok(service
        .0
        .discover_available(repos)
        .await
        .map_err(|e| e.to_string())?)
}

/// 检查 Skills 更新
#[command_api(state = "service, app_state")]
pub async fn check_skill_updates(
    service: &SkillServiceState,
    app_state: &AppState,
) -> Result<Vec<SkillUpdateInfo>, AppError> {
    Ok(service
        .0
        .check_updates(&app_state.db)
        .await
        .map_err(|e| e.to_string())?)
}

/// 更新单个 Skill
#[command_api(state = "service, app_state")]
pub async fn update_skill(
    id: String,
    service: &SkillServiceState,
    app_state: &AppState,
) -> Result<InstalledSkill, AppError> {
    Ok(service
        .0
        .update_skill(&app_state.db, &id)
        .await
        .map_err(|e| e.to_string())?)
}

/// 迁移 Skill 存储位置
#[command_api]
pub async fn migrate_skill_storage(
    target: SkillStorageLocation,
    state: &AppState,
) -> Result<MigrationResult, AppError> {
    Ok(SkillService::migrate_storage(&state.db, target).map_err(|e| e.to_string())?)
}

/// 搜索 skills.sh 公共目录
#[command_api]
pub async fn search_skills_sh(
    query: String,
    limit: usize,
    offset: usize,
) -> Result<SkillsShSearchResult, AppError> {
    Ok(SkillService::search_skills_sh(&query, limit, offset)
        .await
        .map_err(|e| e.to_string())?)
}

// ========== 兼容旧 API 的命令 ==========

/// 获取技能列表（兼容旧 API）
#[command_api(state = "service, app_state")]
pub async fn get_skills(
    service: &SkillServiceState,
    app_state: &AppState,
) -> Result<Vec<Skill>, AppError> {
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    Ok(service
        .0
        .list_skills(repos, &app_state.db)
        .await
        .map_err(|e| e.to_string())?)
}

/// 获取指定应用的技能列表（兼容旧 API）
#[command_api(state = "service, app_state")]
pub async fn get_skills_for_app(
    app: String,
    service: &SkillServiceState,
    app_state: &AppState,
) -> Result<Vec<Skill>, AppError> {
    // 新版本不再区分应用，统一返回所有技能
    let _ = parse_app_type(&app)?; // 验证 app 参数有效
    get_skills_impl(service, app_state).await
}

/// 安装技能（兼容旧 API）
#[command_api(state = "service, app_state")]
pub async fn install_skill(
    directory: String,
    service: &SkillServiceState,
    app_state: &AppState,
) -> Result<bool, AppError> {
    install_skill_for_app_impl("claude".to_string(), directory, service, app_state).await
}

/// 安装指定应用的技能（兼容旧 API）
#[command_api(state = "service, app_state")]
pub async fn install_skill_for_app(
    app: String,
    directory: String,
    service: &SkillServiceState,
    app_state: &AppState,
) -> Result<bool, AppError> {
    let app_type = parse_app_type(&app)?;

    // 先获取技能信息
    let repos = app_state.db.get_skill_repos().map_err(|e| e.to_string())?;
    let skills = service
        .0
        .discover_available(repos)
        .await
        .map_err(|e| e.to_string())?;

    let skill = skills
        .into_iter()
        .find(|s| {
            let install_name = std::path::Path::new(&s.directory)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| s.directory.clone());
            install_name.eq_ignore_ascii_case(&directory)
                || s.directory.eq_ignore_ascii_case(&directory)
        })
        .ok_or_else(|| {
            format_skill_error(
                "SKILL_NOT_FOUND",
                &[("directory", &directory)],
                Some("checkRepoUrl"),
            )
        })?;

    service
        .0
        .install(&app_state.db, &skill, &app_type)
        .await
        .map_err(|e| e.to_string())?;

    Ok(true)
}

/// 卸载技能（兼容旧 API）
#[command_api]
pub fn uninstall_skill(
    directory: String,
    state: &AppState,
) -> Result<SkillUninstallResult, AppError> {
    uninstall_skill_for_app_impl("claude".to_string(), directory, state)
}

/// 卸载指定应用的技能（兼容旧 API）
#[command_api]
pub fn uninstall_skill_for_app(
    app: String,
    directory: String,
    state: &AppState,
) -> Result<SkillUninstallResult, AppError> {
    let _ = parse_app_type(&app)?; // 验证参数

    // 通过 directory 找到对应的 skill id
    let skills = SkillService::get_all_installed(&state.db).map_err(|e| e.to_string())?;

    let skill = skills
        .into_iter()
        .find(|s| s.directory.eq_ignore_ascii_case(&directory))
        .ok_or_else(|| format!("未找到已安装的 Skill: {directory}"))?;

    Ok(SkillService::uninstall(&state.db, &skill.id).map_err(|e| e.to_string())?)
}

// ========== 仓库管理命令 ==========

/// 获取技能仓库列表
#[command_api]
pub fn get_skill_repos(state: &AppState) -> Result<Vec<SkillRepo>, AppError> {
    Ok(state.db.get_skill_repos().map_err(|e| e.to_string())?)
}

/// 添加技能仓库
#[command_api]
pub fn add_skill_repo(repo: SkillRepo, state: &AppState) -> Result<bool, AppError> {
    // 整个结构体由前端反序列化而来，owner/name/branch 会被拼进归档下载 URL。
    // 主防线在 download_repo，这里让非法值当场报错而不是沉淀进表。
    SkillService::validate_repo_ref(&repo.owner, &repo.name, &repo.branch)
        .map_err(|e| e.to_string())?;
    state.db.save_skill_repo(&repo).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 删除技能仓库
#[command_api]
pub fn remove_skill_repo(owner: String, name: String, state: &AppState) -> Result<bool, AppError> {
    state
        .db
        .delete_skill_repo(&owner, &name)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 从 ZIP 文件安装 Skills
#[command_api]
pub fn install_skills_from_zip(
    file_path: String,
    current_app: String,
    state: &AppState,
) -> Result<Vec<InstalledSkill>, AppError> {
    let app_type = parse_app_type(&current_app)?;
    let path = std::path::Path::new(&file_path);

    Ok(SkillService::install_from_zip(&state.db, path, &app_type).map_err(|e| e.to_string())?)
}
