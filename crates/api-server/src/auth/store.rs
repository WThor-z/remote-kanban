use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::types::{ApiKey, HostEnrollment, Membership, Organization, User};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct OrganizationsFile {
    organizations: Vec<Organization>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct UsersFile {
    users: Vec<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MembershipsFile {
    memberships: Vec<Membership>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ApiKeysFile {
    api_keys: Vec<ApiKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct HostEnrollmentsFile {
    hosts: Vec<HostEnrollment>,
}

#[derive(Debug, Clone)]
struct AuthState {
    organizations: Vec<Organization>,
    users: Vec<User>,
    memberships: Vec<Membership>,
    #[allow(dead_code)]
    api_keys: Vec<ApiKey>,
    host_enrollments: Vec<HostEnrollment>,
}

pub struct AuthStore {
    organizations_path: PathBuf,
    users_path: PathBuf,
    memberships_path: PathBuf,
    api_keys_path: PathBuf,
    hosts_path: PathBuf,
    state: RwLock<AuthState>,
}

fn default_org_id() -> String {
    std::env::var("VK_DEFAULT_ORG_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default-org".to_string())
}

fn default_org_name() -> String {
    std::env::var("VK_DEFAULT_ORG_NAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Default Organization".to_string())
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

async fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    fs::write(path, bytes).await
}

async fn read_json_or_default<T>(path: &Path, default: T) -> std::io::Result<T>
where
    T: DeserializeOwned + Serialize + Clone,
{
    match fs::read(path).await {
        Ok(bytes) => serde_json::from_slice::<T>(&bytes).or(Ok(default)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            write_json_pretty(path, &default).await?;
            Ok(default)
        }
        Err(err) => Err(err),
    }
}

fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

impl AuthStore {
    pub async fn new(root_dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&root_dir).await?;

        let organizations_path = root_dir.join("organizations.json");
        let users_path = root_dir.join("users.json");
        let memberships_path = root_dir.join("memberships.json");
        let api_keys_path = root_dir.join("api_keys.json");
        let hosts_path = root_dir.join("hosts.json");

        let org_file = read_json_or_default(
            &organizations_path,
            OrganizationsFile {
                organizations: Vec::new(),
            },
        )
        .await?;
        let users_file = read_json_or_default(&users_path, UsersFile { users: Vec::new() }).await?;
        let memberships_file = read_json_or_default(
            &memberships_path,
            MembershipsFile {
                memberships: Vec::new(),
            },
        )
        .await?;
        let api_keys_file = read_json_or_default(
            &api_keys_path,
            ApiKeysFile {
                api_keys: Vec::new(),
            },
        )
        .await?;
        let hosts_file =
            read_json_or_default(&hosts_path, HostEnrollmentsFile { hosts: Vec::new() }).await?;

        let store = Self {
            organizations_path,
            users_path,
            memberships_path,
            api_keys_path,
            hosts_path,
            state: RwLock::new(AuthState {
                organizations: org_file.organizations,
                users: users_file.users,
                memberships: memberships_file.memberships,
                api_keys: api_keys_file.api_keys,
                host_enrollments: hosts_file.hosts,
            }),
        };

        // Ensure default organization always exists.
        let _ = store
            .ensure_organization(&default_org_id(), &default_org_name())
            .await;

        Ok(store)
    }

    async fn persist_organizations(&self, organizations: &[Organization]) -> Result<(), String> {
        write_json_pretty(
            &self.organizations_path,
            &OrganizationsFile {
                organizations: organizations.to_vec(),
            },
        )
        .await
        .map_err(|err| format!("Failed to persist organizations: {}", err))
    }

    async fn persist_users(&self, users: &[User]) -> Result<(), String> {
        write_json_pretty(
            &self.users_path,
            &UsersFile {
                users: users.to_vec(),
            },
        )
        .await
        .map_err(|err| format!("Failed to persist users: {}", err))
    }

    async fn persist_memberships(&self, memberships: &[Membership]) -> Result<(), String> {
        write_json_pretty(
            &self.memberships_path,
            &MembershipsFile {
                memberships: memberships.to_vec(),
            },
        )
        .await
        .map_err(|err| format!("Failed to persist memberships: {}", err))
    }

    async fn persist_hosts(&self, hosts: &[HostEnrollment]) -> Result<(), String> {
        write_json_pretty(
            &self.hosts_path,
            &HostEnrollmentsFile {
                hosts: hosts.to_vec(),
            },
        )
        .await
        .map_err(|err| format!("Failed to persist host enrollments: {}", err))
    }

    #[allow(dead_code)]
    async fn persist_api_keys(&self, api_keys: &[ApiKey]) -> Result<(), String> {
        write_json_pretty(
            &self.api_keys_path,
            &ApiKeysFile {
                api_keys: api_keys.to_vec(),
            },
        )
        .await
        .map_err(|err| format!("Failed to persist api keys: {}", err))
    }

    pub async fn ensure_organization(
        &self,
        org_id: &str,
        name: &str,
    ) -> Result<Organization, String> {
        let normalized_id = org_id.trim();
        if normalized_id.is_empty() {
            return Err("Organization id is required".to_string());
        }

        let normalized_name = name.trim();
        if normalized_name.is_empty() {
            return Err("Organization name is required".to_string());
        }

        let mut state = self.state.write().await;
        if let Some(existing) = state
            .organizations
            .iter()
            .find(|org| org.id == normalized_id)
            .cloned()
        {
            return Ok(existing);
        }

        let org = Organization {
            id: normalized_id.to_string(),
            name: normalized_name.to_string(),
            created_at: now_iso(),
            disabled: false,
        };
        state.organizations.push(org.clone());
        self.persist_organizations(&state.organizations).await?;
        Ok(org)
    }

    pub async fn create_organization(&self, name: &str) -> Result<Organization, String> {
        let normalized_name = name.trim();
        if normalized_name.is_empty() {
            return Err("Organization name is required".to_string());
        }

        let mut state = self.state.write().await;
        let org = Organization {
            id: format!("org-{}", Uuid::new_v4()),
            name: normalized_name.to_string(),
            created_at: now_iso(),
            disabled: false,
        };
        state.organizations.push(org.clone());
        self.persist_organizations(&state.organizations).await?;
        Ok(org)
    }

    pub async fn list_organizations_for_user(&self, user_id: &str) -> Vec<Organization> {
        let state = self.state.read().await;
        let org_ids = state
            .memberships
            .iter()
            .filter(|membership| membership.user_id == user_id)
            .map(|membership| membership.org_id.as_str())
            .collect::<std::collections::HashSet<_>>();

        state
            .organizations
            .iter()
            .filter(|org| org_ids.contains(org.id.as_str()) && !org.disabled)
            .cloned()
            .collect()
    }

    #[allow(dead_code)]
    pub async fn get_organization(&self, org_id: &str) -> Option<Organization> {
        self.state
            .read()
            .await
            .organizations
            .iter()
            .find(|org| org.id == org_id && !org.disabled)
            .cloned()
    }

    pub async fn create_user(
        &self,
        email: &str,
        password: &str,
        name: &str,
    ) -> Result<User, String> {
        let normalized_email = normalize_email(email);
        if normalized_email.is_empty() {
            return Err("Email is required".to_string());
        }
        if password.trim().len() < 6 {
            return Err("Password must be at least 6 characters".to_string());
        }

        let normalized_name = name.trim();
        if normalized_name.is_empty() {
            return Err("Name is required".to_string());
        }

        let mut state = self.state.write().await;
        if state
            .users
            .iter()
            .any(|user| user.email.eq_ignore_ascii_case(&normalized_email))
        {
            return Err("Email already registered".to_string());
        }

        let user = User {
            id: format!("usr-{}", Uuid::new_v4()),
            email: normalized_email,
            name: normalized_name.to_string(),
            password_hash: hash_password(password),
            created_at: now_iso(),
            disabled: false,
        };
        state.users.push(user.clone());
        self.persist_users(&state.users).await?;
        Ok(user)
    }

    pub async fn authenticate_user(&self, email: &str, password: &str) -> Option<User> {
        let normalized_email = normalize_email(email);
        let password_hash = hash_password(password);

        self.state
            .read()
            .await
            .users
            .iter()
            .find(|user| {
                !user.disabled
                    && user.email.eq_ignore_ascii_case(&normalized_email)
                    && user.password_hash == password_hash
            })
            .cloned()
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> Option<User> {
        self.state
            .read()
            .await
            .users
            .iter()
            .find(|user| user.id == user_id && !user.disabled)
            .cloned()
    }

    pub async fn ensure_membership(
        &self,
        user_id: &str,
        org_id: &str,
        role: &str,
    ) -> Result<Membership, String> {
        let normalized_role = role.trim().to_ascii_lowercase();
        if normalized_role.is_empty() {
            return Err("Role is required".to_string());
        }

        let mut state = self.state.write().await;
        if let Some(existing) = state
            .memberships
            .iter()
            .find(|membership| membership.user_id == user_id && membership.org_id == org_id)
            .cloned()
        {
            return Ok(existing);
        }

        let membership = Membership {
            id: format!("mem-{}", Uuid::new_v4()),
            user_id: user_id.to_string(),
            org_id: org_id.to_string(),
            role: normalized_role,
            created_at: now_iso(),
        };
        state.memberships.push(membership.clone());
        self.persist_memberships(&state.memberships).await?;
        Ok(membership)
    }

    #[allow(dead_code)]
    pub async fn get_membership(&self, user_id: &str, org_id: &str) -> Option<Membership> {
        self.state
            .read()
            .await
            .memberships
            .iter()
            .find(|membership| membership.user_id == user_id && membership.org_id == org_id)
            .cloned()
    }

    pub async fn list_memberships_for_user(&self, user_id: &str) -> Vec<Membership> {
        self.state
            .read()
            .await
            .memberships
            .iter()
            .filter(|membership| membership.user_id == user_id)
            .cloned()
            .collect()
    }

    pub async fn upsert_host_enrollment(
        &self,
        org_id: &str,
        host_id: &str,
    ) -> Result<HostEnrollment, String> {
        let normalized_org = org_id.trim();
        let normalized_host = host_id.trim();
        if normalized_org.is_empty() || normalized_host.is_empty() {
            return Err("orgId and hostId are required".to_string());
        }

        let mut state = self.state.write().await;
        if let Some(existing) = state
            .host_enrollments
            .iter_mut()
            .find(|host| host.org_id == normalized_org && host.host_id == normalized_host)
        {
            existing.enabled = true;
            existing.updated_at = now_iso();
            let updated = existing.clone();
            self.persist_hosts(&state.host_enrollments).await?;
            return Ok(updated);
        }

        let now = now_iso();
        let enrollment = HostEnrollment {
            host_id: normalized_host.to_string(),
            org_id: normalized_org.to_string(),
            token_version: 1,
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        };

        state.host_enrollments.push(enrollment.clone());
        self.persist_hosts(&state.host_enrollments).await?;
        Ok(enrollment)
    }

    pub async fn rotate_host_token(
        &self,
        org_id: &str,
        host_id: &str,
    ) -> Result<HostEnrollment, String> {
        let mut state = self.state.write().await;
        let Some(existing) = state
            .host_enrollments
            .iter_mut()
            .find(|host| host.org_id == org_id && host.host_id == host_id)
        else {
            return Err("Host enrollment not found".to_string());
        };

        existing.enabled = true;
        existing.token_version = existing.token_version.saturating_add(1);
        existing.updated_at = now_iso();
        let updated = existing.clone();
        self.persist_hosts(&state.host_enrollments).await?;
        Ok(updated)
    }

    pub async fn disable_host(
        &self,
        org_id: &str,
        host_id: &str,
    ) -> Result<HostEnrollment, String> {
        let mut state = self.state.write().await;
        let Some(existing) = state
            .host_enrollments
            .iter_mut()
            .find(|host| host.org_id == org_id && host.host_id == host_id)
        else {
            return Err("Host enrollment not found".to_string());
        };

        existing.enabled = false;
        existing.updated_at = now_iso();
        let updated = existing.clone();
        self.persist_hosts(&state.host_enrollments).await?;
        Ok(updated)
    }

    #[allow(dead_code)]
    pub async fn get_host_enrollment(&self, host_id: &str) -> Option<HostEnrollment> {
        self.state
            .read()
            .await
            .host_enrollments
            .iter()
            .find(|host| host.host_id == host_id)
            .cloned()
    }

    pub async fn get_host_enrollment_in_org(
        &self,
        org_id: &str,
        host_id: &str,
    ) -> Option<HostEnrollment> {
        self.state
            .read()
            .await
            .host_enrollments
            .iter()
            .find(|host| host.org_id == org_id && host.host_id == host_id)
            .cloned()
    }

    pub async fn list_hosts_for_org(&self, org_id: &str) -> Vec<HostEnrollment> {
        self.state
            .read()
            .await
            .host_enrollments
            .iter()
            .filter(|host| host.org_id == org_id && host.enabled)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn user_registration_and_authentication_work() {
        let temp_dir = TempDir::new().unwrap();
        let store = AuthStore::new(temp_dir.path().to_path_buf()).await.unwrap();

        let user = store
            .create_user("dev@example.com", "dev-pass", "Dev")
            .await
            .unwrap();

        let authenticated = store
            .authenticate_user("dev@example.com", "dev-pass")
            .await
            .unwrap();
        assert_eq!(authenticated.id, user.id);
    }

    #[tokio::test]
    async fn host_enrollment_rotate_and_disable_work() {
        let temp_dir = TempDir::new().unwrap();
        let store = AuthStore::new(temp_dir.path().to_path_buf()).await.unwrap();

        let enrolled = store
            .upsert_host_enrollment("org-test", "host-a")
            .await
            .unwrap();
        assert_eq!(enrolled.token_version, 1);
        assert!(enrolled.enabled);

        let rotated = store.rotate_host_token("org-test", "host-a").await.unwrap();
        assert_eq!(rotated.token_version, 2);

        let disabled = store.disable_host("org-test", "host-a").await.unwrap();
        assert!(!disabled.enabled);
    }
}
