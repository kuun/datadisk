//! Permission module using Casbin
//!
//! Implements RBAC permission management with Casbin

use casbin::{CoreApi, DefaultModel, Enforcer, MgmtApi};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::entity::casbin_rule;

/// Permission constants
pub mod perm {
    pub const FILE: &str = "file";
    pub const CONTACTS: &str = "contacts";
    pub const ROLE: &str = "role";
    pub const GROUP: &str = "group";
    pub const AUDIT: &str = "audit";

    /// All permissions
    pub const ALL: [&str; 5] = [FILE, CONTACTS, ROLE, GROUP, AUDIT];
}

/// Action constants
pub mod action {
    pub const ACCESS: &str = "access";
}

/// Permission enforcer wrapper
#[derive(Clone)]
pub struct PermissionEnforcer {
    enforcer: Arc<RwLock<Enforcer>>,
    db: DatabaseConnection,
}

impl PermissionEnforcer {
    /// Create a new permission enforcer
    pub async fn new(db: DatabaseConnection, model_path: &str) -> anyhow::Result<Self> {
        let model = DefaultModel::from_file(model_path).await?;
        let enforcer = Enforcer::new(model, ()).await?;

        let perm_enforcer = Self {
            enforcer: Arc::new(RwLock::new(enforcer)),
            db,
        };

        // Load policies from database
        perm_enforcer.load_policies().await?;

        Ok(perm_enforcer)
    }

    /// Load all policies from database
    pub async fn load_policies(&self) -> anyhow::Result<()> {
        let rules = casbin_rule::Entity::find()
            .all(&self.db)
            .await?;

        let mut enforcer = self.enforcer.write().await;
        enforcer.clear_policy().await?;

        for rule in rules {
            let policy = rule.to_policy_vec();
            if rule.ptype == "p" {
                let _ = enforcer.add_policy(policy).await;
            } else if rule.ptype == "g" {
                let _ = enforcer.add_grouping_policy(policy).await;
            }
        }

        Ok(())
    }

    /// Check if user has permission
    pub async fn check(&self, user: &str, obj: &str, act: &str) -> bool {
        let enforcer = self.enforcer.read().await;
        enforcer.enforce((user, obj, act)).unwrap_or(false)
    }

    /// Check if user has access to a resource
    pub async fn can_access(&self, user: &str, resource: &str) -> bool {
        self.check(user, resource, action::ACCESS).await
    }

    /// Get all permissions for a user
    pub async fn get_user_permissions(&self, user: &str) -> Vec<String> {
        let enforcer = self.enforcer.read().await;
        let mut permissions = Vec::new();

        for perm in perm::ALL {
            if enforcer.enforce((user, perm, action::ACCESS)).unwrap_or(false) {
                permissions.push(perm.to_string());
            }
        }

        permissions
    }

    /// Get direct permissions assigned to user (not via roles)
    pub async fn get_direct_permissions(&self, user: &str) -> anyhow::Result<Vec<String>> {
        let rules = casbin_rule::Entity::find()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(user))
            .filter(casbin_rule::Column::V2.eq(Some(action::ACCESS.to_string())))
            .all(&self.db)
            .await?;

        Ok(rules.into_iter().map(|r| r.v1).collect())
    }

    /// Add policy: user can access resource
    pub async fn add_permission(&self, user: &str, resource: &str) -> anyhow::Result<()> {
        // Add to database
        let rule = casbin_rule::ActiveModel {
            ptype: Set("p".to_string()),
            v0: Set(user.to_string()),
            v1: Set(resource.to_string()),
            v2: Set(Some(action::ACCESS.to_string())),
            ..Default::default()
        };
        rule.insert(&self.db).await?;

        // Add to enforcer
        let mut enforcer = self.enforcer.write().await;
        enforcer.add_policy(vec![
            user.to_string(),
            resource.to_string(),
            action::ACCESS.to_string(),
        ]).await?;

        Ok(())
    }

    /// Remove policy
    pub async fn remove_permission(&self, user: &str, resource: &str) -> anyhow::Result<()> {
        // Remove from database
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(user))
            .filter(casbin_rule::Column::V1.eq(resource))
            .filter(casbin_rule::Column::V2.eq(action::ACCESS))
            .exec(&self.db)
            .await?;

        // Remove from enforcer
        let mut enforcer = self.enforcer.write().await;
        enforcer.remove_policy(vec![
            user.to_string(),
            resource.to_string(),
            action::ACCESS.to_string(),
        ]).await?;

        Ok(())
    }

    /// Add user to role
    pub async fn add_role(&self, user: &str, role: &str) -> anyhow::Result<()> {
        // Add to database
        let rule = casbin_rule::ActiveModel {
            ptype: Set("g".to_string()),
            v0: Set(user.to_string()),
            v1: Set(role.to_string()),
            v2: Set(None),
            ..Default::default()
        };
        rule.insert(&self.db).await?;

        // Add to enforcer
        let mut enforcer = self.enforcer.write().await;
        enforcer.add_grouping_policy(vec![
            user.to_string(),
            role.to_string(),
        ]).await?;

        Ok(())
    }

    /// Remove user from role
    pub async fn remove_role(&self, user: &str, role: &str) -> anyhow::Result<()> {
        // Remove from database
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V0.eq(user))
            .filter(casbin_rule::Column::V1.eq(role))
            .exec(&self.db)
            .await?;

        // Remove from enforcer
        let mut enforcer = self.enforcer.write().await;
        enforcer.remove_grouping_policy(vec![
            user.to_string(),
            role.to_string(),
        ]).await?;

        Ok(())
    }

    /// Grant all permissions to user
    pub async fn grant_all_permissions(&self, user: &str) -> anyhow::Result<()> {
        for perm in perm::ALL {
            self.add_permission(user, perm).await?;
        }
        Ok(())
    }

    /// Revoke all permissions from user
    pub async fn revoke_all_permissions(&self, user: &str) -> anyhow::Result<()> {
        // Remove all policies for user from database
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::V0.eq(user))
            .exec(&self.db)
            .await?;

        // Reload policies
        self.load_policies().await?;

        Ok(())
    }

    /// Set permissions for user (replace existing)
    pub async fn set_permissions(&self, user: &str, permissions: &[&str]) -> anyhow::Result<()> {
        // Remove all existing policies for user
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(user))
            .exec(&self.db)
            .await?;

        // Add new policies
        for perm in permissions {
            let rule = casbin_rule::ActiveModel {
                ptype: Set("p".to_string()),
                v0: Set(user.to_string()),
                v1: Set(perm.to_string()),
                v2: Set(Some(action::ACCESS.to_string())),
                ..Default::default()
            };
            rule.insert(&self.db).await?;
        }

        // Reload enforcer
        self.load_policies().await?;

        Ok(())
    }

    // ==================== Role Management ====================

    /// Role name prefix to distinguish from usernames
    pub const ROLE_PREFIX: &'static str = "role:";
    /// Department role prefix
    pub const DEPT_PREFIX: &'static str = "dept:";

    /// Get prefixed role name
    fn role_name(role: &str) -> String {
        format!("{}{}", Self::ROLE_PREFIX, role)
    }

    /// Check if a subject is a role (not a user)
    fn is_role(subject: &str) -> bool {
        subject.starts_with(Self::ROLE_PREFIX)
    }

    /// Extract role name from prefixed string
    fn extract_role_name(prefixed: &str) -> &str {
        prefixed.strip_prefix(Self::ROLE_PREFIX).unwrap_or(prefixed)
    }

    fn dept_role_name(dept_id: i64) -> String {
        format!("{}{}", Self::DEPT_PREFIX, dept_id)
    }

    /// Create a new role with permissions
    pub async fn create_role(&self, role: &str, permissions: &[&str]) -> anyhow::Result<()> {
        let role_name = Self::role_name(role);

        // Add role permissions (p policies)
        for perm in permissions {
            let rule = casbin_rule::ActiveModel {
                ptype: Set("p".to_string()),
                v0: Set(role_name.clone()),
                v1: Set(perm.to_string()),
                v2: Set(Some(action::ACCESS.to_string())),
                ..Default::default()
            };
            rule.insert(&self.db).await?;
        }

        // Reload enforcer
        self.load_policies().await?;

        Ok(())
    }

    /// Get all roles with their permissions
    pub async fn get_all_roles(&self) -> anyhow::Result<Vec<RoleInfo>> {
        let rules = casbin_rule::Entity::find()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .all(&self.db)
            .await?;

        // Group permissions by role
        let mut role_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

        for rule in rules {
            if Self::is_role(&rule.v0) {
                let role_name = Self::extract_role_name(&rule.v0).to_string();
                role_map
                    .entry(role_name)
                    .or_default()
                    .push(rule.v1.clone());
            }
        }

        let roles: Vec<RoleInfo> = role_map
            .into_iter()
            .map(|(name, permissions)| RoleInfo { name, permissions, description: None })
            .collect();

        Ok(roles)
    }

    /// Get permissions for a specific role
    pub async fn get_role_permissions(&self, role: &str) -> anyhow::Result<Vec<String>> {
        let role_name = Self::role_name(role);

        let rules = casbin_rule::Entity::find()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(&role_name))
            .all(&self.db)
            .await?;

        let permissions: Vec<String> = rules.into_iter().map(|r| r.v1).collect();
        Ok(permissions)
    }

    /// Update role permissions (replace existing)
    pub async fn update_role_permissions(&self, role: &str, permissions: &[&str]) -> anyhow::Result<()> {
        let role_name = Self::role_name(role);

        // Remove all existing permissions for role
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(&role_name))
            .exec(&self.db)
            .await?;

        // Add new permissions
        for perm in permissions {
            let rule = casbin_rule::ActiveModel {
                ptype: Set("p".to_string()),
                v0: Set(role_name.clone()),
                v1: Set(perm.to_string()),
                v2: Set(Some(action::ACCESS.to_string())),
                ..Default::default()
            };
            rule.insert(&self.db).await?;
        }

        // Reload enforcer
        self.load_policies().await?;

        Ok(())
    }

    /// Update role (supports renaming and changing permissions)
    pub async fn update_role(&self, old_name: &str, new_name: &str, permissions: &[&str]) -> anyhow::Result<()> {
        let old_role_name = Self::role_name(old_name);
        let new_role_name = Self::role_name(new_name);

        // If renaming, update all user-role assignments
        if old_name != new_name {
            // Get all users with this role
            let users = self.get_role_users(old_name).await?;

            // Delete old role permissions
            casbin_rule::Entity::delete_many()
                .filter(casbin_rule::Column::Ptype.eq("p"))
                .filter(casbin_rule::Column::V0.eq(&old_role_name))
                .exec(&self.db)
                .await?;

            // Delete old user-role assignments
            casbin_rule::Entity::delete_many()
                .filter(casbin_rule::Column::Ptype.eq("g"))
                .filter(casbin_rule::Column::V1.eq(&old_role_name))
                .exec(&self.db)
                .await?;

            // Create new role with permissions
            for perm in permissions {
                let rule = casbin_rule::ActiveModel {
                    ptype: Set("p".to_string()),
                    v0: Set(new_role_name.clone()),
                    v1: Set(perm.to_string()),
                    v2: Set(Some(action::ACCESS.to_string())),
                    ..Default::default()
                };
                rule.insert(&self.db).await?;
            }

            // Re-assign users to new role
            for user in users {
                let rule = casbin_rule::ActiveModel {
                    ptype: Set("g".to_string()),
                    v0: Set(user),
                    v1: Set(new_role_name.clone()),
                    v2: Set(None),
                    ..Default::default()
                };
                rule.insert(&self.db).await?;
            }
        } else {
            // Just update permissions
            self.update_role_permissions(old_name, permissions).await?;
            return Ok(());
        }

        // Reload enforcer
        self.load_policies().await?;

        Ok(())
    }

    /// Delete a role and all its associations
    pub async fn delete_role(&self, role: &str) -> anyhow::Result<()> {
        let role_name = Self::role_name(role);

        // Remove role permissions (p policies)
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(&role_name))
            .exec(&self.db)
            .await?;

        // Remove user-role associations (g policies)
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V1.eq(&role_name))
            .exec(&self.db)
            .await?;

        // Reload enforcer
        self.load_policies().await?;

        Ok(())
    }

    /// Assign user to a role
    pub async fn assign_user_role(&self, user: &str, role: &str) -> anyhow::Result<()> {
        let role_name = Self::role_name(role);

        // Check if assignment already exists
        let existing = casbin_rule::Entity::find()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V0.eq(user))
            .filter(casbin_rule::Column::V1.eq(&role_name))
            .one(&self.db)
            .await?;

        if existing.is_some() {
            return Ok(()); // Already assigned
        }

        // Add to database
        let rule = casbin_rule::ActiveModel {
            ptype: Set("g".to_string()),
            v0: Set(user.to_string()),
            v1: Set(role_name.clone()),
            v2: Set(None),
            ..Default::default()
        };
        rule.insert(&self.db).await?;

        // Add to enforcer
        let mut enforcer = self.enforcer.write().await;
        enforcer.add_grouping_policy(vec![user.to_string(), role_name]).await?;

        Ok(())
    }

    /// Remove user from a role
    pub async fn remove_user_role(&self, user: &str, role: &str) -> anyhow::Result<()> {
        let role_name = Self::role_name(role);

        // Remove from database
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V0.eq(user))
            .filter(casbin_rule::Column::V1.eq(&role_name))
            .exec(&self.db)
            .await?;

        // Remove from enforcer
        let mut enforcer = self.enforcer.write().await;
        enforcer.remove_grouping_policy(vec![user.to_string(), role_name]).await?;

        Ok(())
    }

    /// Get user's assigned role (returns first role if multiple)
    pub async fn get_user_role(&self, user: &str) -> anyhow::Result<Option<String>> {
        let rule = casbin_rule::Entity::find()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V0.eq(user))
            .filter(casbin_rule::Column::V1.starts_with(Self::ROLE_PREFIX))
            .one(&self.db)
            .await?;

        Ok(rule.and_then(|r| {
            Some(Self::extract_role_name(&r.v1).to_string())
        }))
    }

    /// Get all users assigned to a role
    pub async fn get_role_users(&self, role: &str) -> anyhow::Result<Vec<String>> {
        let role_name = Self::role_name(role);

        let rules = casbin_rule::Entity::find()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V1.eq(&role_name))
            .all(&self.db)
            .await?;

        let users: Vec<String> = rules.into_iter().map(|r| r.v0).collect();
        Ok(users)
    }

    /// Set user's role (replace existing role)
    pub async fn set_user_role(&self, user: &str, role: Option<&str>) -> anyhow::Result<()> {
        // Remove all existing app role assignments for user (keep department roles)
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V0.eq(user))
            .filter(casbin_rule::Column::V1.starts_with(Self::ROLE_PREFIX))
            .exec(&self.db)
            .await?;

        // Add new role if specified
        if let Some(role) = role {
            let role_name = Self::role_name(role);
            let rule = casbin_rule::ActiveModel {
                ptype: Set("g".to_string()),
                v0: Set(user.to_string()),
                v1: Set(role_name),
                v2: Set(None),
                ..Default::default()
            };
            rule.insert(&self.db).await?;
        }

        // Reload enforcer
        self.load_policies().await?;

        Ok(())
    }

    /// Check if role exists
    pub async fn role_exists(&self, role: &str) -> anyhow::Result<bool> {
        let role_name = Self::role_name(role);

        let exists = casbin_rule::Entity::find()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(&role_name))
            .one(&self.db)
            .await?;

        Ok(exists.is_some())
    }

    /// Create default roles if not exist
    pub async fn ensure_default_roles(&self) -> anyhow::Result<()> {
        // Admin role with all permissions
        if !self.role_exists("admin").await? {
            self.create_role("admin", &perm::ALL).await?;
            tracing::info!("Created default role: admin");
        }

        // User role with basic permissions
        if !self.role_exists("user").await? {
            self.create_role("user", &[perm::FILE, perm::GROUP]).await?;
            tracing::info!("Created default role: user");
        }

        Ok(())
    }

    // ==================== Department Permissions ====================

    /// Set department permissions (replace existing)
    pub async fn set_department_permissions(&self, dept_id: i64, permissions: &[&str]) -> anyhow::Result<()> {
        let role_name = Self::dept_role_name(dept_id);

        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(&role_name))
            .exec(&self.db)
            .await?;

        for perm in permissions {
            let rule = casbin_rule::ActiveModel {
                ptype: Set("p".to_string()),
                v0: Set(role_name.clone()),
                v1: Set(perm.to_string()),
                v2: Set(Some(action::ACCESS.to_string())),
                ..Default::default()
            };
            rule.insert(&self.db).await?;
        }

        self.load_policies().await?;
        Ok(())
    }

    /// Get department permissions
    pub async fn get_department_permissions(&self, dept_id: i64) -> anyhow::Result<Vec<String>> {
        let role_name = Self::dept_role_name(dept_id);
        let rules = casbin_rule::Entity::find()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(&role_name))
            .all(&self.db)
            .await?;

        Ok(rules.into_iter().map(|r| r.v1).collect())
    }

    /// Set department parent (role inheritance)
    pub async fn set_department_parent(&self, dept_id: i64, parent_id: Option<i64>) -> anyhow::Result<()> {
        let role_name = Self::dept_role_name(dept_id);

        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V0.eq(&role_name))
            .filter(casbin_rule::Column::V1.starts_with(Self::DEPT_PREFIX))
            .exec(&self.db)
            .await?;

        if let Some(parent_id) = parent_id {
            if parent_id > 0 {
                let parent_role = Self::dept_role_name(parent_id);
                let rule = casbin_rule::ActiveModel {
                    ptype: Set("g".to_string()),
                    v0: Set(role_name.clone()),
                    v1: Set(parent_role),
                    v2: Set(None),
                    ..Default::default()
                };
                rule.insert(&self.db).await?;
            }
        }

        self.load_policies().await?;
        Ok(())
    }

    /// Assign user to department (used for inherited permissions)
    pub async fn set_user_department(&self, user: &str, dept_id: i64) -> anyhow::Result<()> {
        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(casbin_rule::Column::V0.eq(user))
            .filter(casbin_rule::Column::V1.starts_with(Self::DEPT_PREFIX))
            .exec(&self.db)
            .await?;

        let role_name = Self::dept_role_name(dept_id);
        let rule = casbin_rule::ActiveModel {
            ptype: Set("g".to_string()),
            v0: Set(user.to_string()),
            v1: Set(role_name),
            v2: Set(None),
            ..Default::default()
        };
        rule.insert(&self.db).await?;

        self.load_policies().await?;
        Ok(())
    }

    /// Remove a department role and related policies
    pub async fn remove_department(&self, dept_id: i64) -> anyhow::Result<()> {
        let role_name = Self::dept_role_name(dept_id);

        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("p"))
            .filter(casbin_rule::Column::V0.eq(&role_name))
            .exec(&self.db)
            .await?;

        casbin_rule::Entity::delete_many()
            .filter(casbin_rule::Column::Ptype.eq("g"))
            .filter(
                casbin_rule::Column::V0
                    .eq(&role_name)
                    .or(casbin_rule::Column::V1.eq(&role_name)),
            )
            .exec(&self.db)
            .await?;

        self.load_policies().await?;
        Ok(())
    }
}

/// Normalize permissions string into sorted, unique list
pub fn normalize_permissions(permissions: &str) -> Vec<String> {
    let valid_perms: std::collections::HashSet<&str> = perm::ALL.iter().copied().collect();

    let mut perms: Vec<String> = permissions
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && valid_perms.contains(*s))
        .map(|s| s.to_string())
        .collect();

    perms.sort();
    perms.dedup();
    perms
}

/// Role information
#[derive(Debug, Clone, serde::Serialize)]
pub struct RoleInfo {
    pub name: String,
    pub permissions: Vec<String>,
    pub description: Option<String>,
}
