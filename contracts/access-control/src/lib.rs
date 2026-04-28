#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN,
    Env, String, Vec,
};

mod test;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    InvalidDidFormat = 1,
    AlreadyInitialized = 2,
    EntityAlreadyRegistered = 3,
    EntityNotFound = 4,
    GrantorNotRegistered = 5,
    GranteeNotRegistered = 6,
    AccessAlreadyGranted = 7,
    NotAuthorizedToRevoke = 8,
    AccessPermissionNotFound = 9,
    ContractNotInitialized = 10,
    OnlyAdminCanDeactivate = 11,
    // Role-based access control errors
    RoleAlreadyGranted = 12,
    RoleNotFound = 13,
    InsufficientRole = 14,
    RoleExpired = 15,
}

/// --------------------
/// Roles
/// --------------------
/// Operational roles that can be granted to addresses. Each role scopes
/// which privileged entry points the holder may call.
///
/// Hierarchy (highest → lowest privilege):
///   Admin > Auditor > PayerReviewer | Provider | EmergencyResponder
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    /// Full administrative control: grant/revoke roles, deactivate entities.
    Admin,
    /// Read-only audit access across all resources.
    Auditor,
    /// Insurance / payer reviewer: may adjudicate and revoke resource access.
    PayerReviewer,
    /// Healthcare provider: may grant resource access to patients.
    Provider,
    /// Emergency responder: may access any resource without prior consent
    /// (break-glass); every use is logged.
    EmergencyResponder,
}

/// A single role assignment stored per (address, role) pair.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleAssignment {
    /// Who granted this role.
    pub granted_by: Address,
    /// Ledger timestamp when the role was granted.
    pub granted_at: u64,
    /// Optional expiry timestamp; 0 means the role never expires.
    pub expires_at: u64,
}

/// --------------------
/// Entity Types
/// --------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityType {
    Hospital,
    Doctor,
    Patient,
    Insurer,
    Admin,
}

/// --------------------
/// Entity Data
/// --------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityData {
    pub entity_type: EntityType,
    pub name: String,
    pub metadata: String,
    pub active: bool,
}

/// --------------------
/// Access Permission
/// --------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessPermission {
    pub resource_id: String,
    pub granted_by: Address,
    pub granted_at: u64,
    pub expires_at: u64, // 0 means no expiration
}

/// --------------------
/// Storage Keys
/// --------------------
#[contracttype]
pub enum DataKey {
    Admin,
    Entity(Address),
    AccessList(Address),    // Entity -> Vec<AccessPermission>
    ResourceAccess(String), // Resource -> Vec<Address> (authorized parties)
    Did(Address),
    /// (address, role) -> RoleAssignment
    RoleAssignment(Address, Role),
}

#[contract]
pub struct AccessControl;

#[contractimpl]
impl AccessControl {
    // -------------------------------------------------------------------------
    // Internal role helpers
    // -------------------------------------------------------------------------

    /// Returns the stored `RoleAssignment` for `(address, role)` if it exists
    /// **and** has not expired. Expired entries are treated as absent.
    fn load_active_role(
        env: &Env,
        address: &Address,
        role: &Role,
    ) -> Option<RoleAssignment> {
        let key = DataKey::RoleAssignment(address.clone(), role.clone());
        let assignment: RoleAssignment = env.storage().persistent().get(&key)?;
        let now = env.ledger().timestamp();
        if assignment.expires_at != 0 && assignment.expires_at <= now {
            return None;
        }
        Some(assignment)
    }

    /// Asserts that `caller` holds `role` (and the role has not expired).
    /// Returns `InsufficientRole` if the check fails.
    fn require_role(
        env: &Env,
        caller: &Address,
        role: &Role,
    ) -> Result<(), ContractError> {
        // Admin always satisfies any role check.
        let admin_opt: Option<Address> = env.storage().persistent().get(&DataKey::Admin);
        if let Some(ref admin) = admin_opt {
            if caller == admin {
                return Ok(());
            }
        }
        if Self::load_active_role(env, caller, role).is_some() {
            return Ok(());
        }
        Err(ContractError::InsufficientRole)
    }

    // -------------------------------------------------------------------------
    // Initialize
    // -------------------------------------------------------------------------

    /// Initialize the contract with an admin
    ///
    /// # Arguments
    /// * `admin` - The admin address for the contract
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().persistent().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &admin);

        // Bootstrap: give the admin the Admin role so role checks are uniform.
        let bootstrap = RoleAssignment {
            granted_by: admin.clone(),
            granted_at: env.ledger().timestamp(),
            expires_at: 0,
        };
        env.storage().persistent().set(
            &DataKey::RoleAssignment(admin.clone(), Role::Admin),
            &bootstrap,
        );

        env.events()
            .publish((symbol_short!("init"), admin), symbol_short!("success"));
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Role management
    // -------------------------------------------------------------------------

    /// Grant `role` to `grantee`. Only an address that itself holds the
    /// `Admin` role (or is the stored admin address) may call this.
    ///
    /// # Arguments
    /// * `granter`    - Must hold the `Admin` role.
    /// * `grantee`    - Address receiving the role.
    /// * `role`       - The role to grant.
    /// * `expires_at` - Expiry timestamp; pass `0` for no expiry.
    pub fn grant_role(
        env: Env,
        granter: Address,
        grantee: Address,
        role: Role,
        expires_at: u64,
    ) -> Result<(), ContractError> {
        granter.require_auth();
        Self::require_role(&env, &granter, &Role::Admin)?;

        let key = DataKey::RoleAssignment(grantee.clone(), role.clone());
        if env.storage().persistent().has(&key) {
            // Allow re-grant only if the existing assignment has expired.
            if Self::load_active_role(&env, &grantee, &role).is_some() {
                return Err(ContractError::RoleAlreadyGranted);
            }
        }

        let assignment = RoleAssignment {
            granted_by: granter.clone(),
            granted_at: env.ledger().timestamp(),
            expires_at,
        };
        env.storage().persistent().set(&key, &assignment);

        env.events().publish(
            (symbol_short!("role_grt"), grantee, role),
            symbol_short!("success"),
        );
        Ok(())
    }

    /// Revoke `role` from `revokee`. Only an address that holds the `Admin`
    /// role may call this.
    ///
    /// # Arguments
    /// * `revoker`  - Must hold the `Admin` role.
    /// * `revokee`  - Address losing the role.
    /// * `role`     - The role to revoke.
    pub fn revoke_role(
        env: Env,
        revoker: Address,
        revokee: Address,
        role: Role,
    ) -> Result<(), ContractError> {
        revoker.require_auth();
        Self::require_role(&env, &revoker, &Role::Admin)?;

        let key = DataKey::RoleAssignment(revokee.clone(), role.clone());
        if !env.storage().persistent().has(&key) {
            return Err(ContractError::RoleNotFound);
        }
        env.storage().persistent().remove(&key);

        env.events().publish(
            (symbol_short!("role_rev"), revokee, role),
            symbol_short!("success"),
        );
        Ok(())
    }

    /// Returns `true` if `address` currently holds `role` (and it has not
    /// expired). Does **not** require any auth — safe to call as a view.
    pub fn has_role(env: Env, address: Address, role: Role) -> bool {
        // Admin address always satisfies any role.
        let admin_opt: Option<Address> = env.storage().persistent().get(&DataKey::Admin);
        if let Some(ref admin) = admin_opt {
            if &address == admin {
                return true;
            }
        }
        Self::load_active_role(&env, &address, &role).is_some()
    }

    /// Returns the full `RoleAssignment` for `(address, role)`, or an error
    /// if the role was never granted or has expired.
    pub fn get_role_assignment(
        env: Env,
        address: Address,
        role: Role,
    ) -> Result<RoleAssignment, ContractError> {
        Self::load_active_role(&env, &address, &role)
            .ok_or(ContractError::RoleNotFound)
    }

    /// Register a new entity in the system
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the entity
    /// * `entity_type` - The type of entity (Hospital, Doctor, Patient, etc.)
    /// * `name` - The name of the entity
    /// * `metadata` - Additional information about the entity
    pub fn register_entity(
        env: Env,
        wallet: Address,
        entity_type: EntityType,
        name: String,
        metadata: String,
    ) -> Result<(), ContractError> {
        wallet.require_auth();

        let key = DataKey::Entity(wallet.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::EntityAlreadyRegistered);
        }

        let entity = EntityData {
            entity_type,
            name,
            metadata,
            active: true,
        };

        env.storage().persistent().set(&key, &entity);

        // Initialize empty access list for the entity
        let empty_access: Vec<AccessPermission> = Vec::new(&env);
        env.storage()
            .persistent()
            .set(&DataKey::AccessList(wallet.clone()), &empty_access);

        env.events()
            .publish((symbol_short!("reg_ent"), wallet), symbol_short!("success"));
        Ok(())
    }

    /// Grant access permission to an entity for a specific resource
    ///
    /// # Arguments
    /// * `grantor` - The address granting access (must be authorized)
    /// * `grantee` - The address receiving access
    /// * `resource_id` - The identifier of the resource
    /// * `expires_at` - Expiration timestamp (0 for no expiration)
    pub fn grant_access(
        env: Env,
        grantor: Address,
        grantee: Address,
        resource_id: String,
        expires_at: u64,
    ) -> Result<(), ContractError> {
        grantor.require_auth();

        // Verify grantor is a registered entity
        let grantor_key = DataKey::Entity(grantor.clone());
        if !env.storage().persistent().has(&grantor_key) {
            return Err(ContractError::GrantorNotRegistered);
        }

        // Verify grantee is a registered entity
        let grantee_key = DataKey::Entity(grantee.clone());
        if !env.storage().persistent().has(&grantee_key) {
            return Err(ContractError::GranteeNotRegistered);
        }

        let permission = AccessPermission {
            resource_id: resource_id.clone(),
            granted_by: grantor.clone(),
            granted_at: env.ledger().timestamp(),
            expires_at,
        };

        // Add permission to grantee's access list
        let access_key = DataKey::AccessList(grantee.clone());
        let mut access_list: Vec<AccessPermission> = env
            .storage()
            .persistent()
            .get(&access_key)
            .unwrap_or(Vec::new(&env));

        // Check if permission already exists for this resource
        let mut exists = false;
        for i in 0..access_list.len() {
            if let Some(existing) = access_list.get(i) {
                if existing.resource_id == resource_id {
                    exists = true;
                    break;
                }
            }
        }
        if exists {
            return Err(ContractError::AccessAlreadyGranted);
        }

        access_list.push_back(permission);
        env.storage().persistent().set(&access_key, &access_list);

        // Add grantee to resource's authorized parties
        let resource_key = DataKey::ResourceAccess(resource_id.clone());
        let mut authorized: Vec<Address> = env
            .storage()
            .persistent()
            .get(&resource_key)
            .unwrap_or(Vec::new(&env));

        authorized.push_back(grantee.clone());
        env.storage().persistent().set(&resource_key, &authorized);

        env.events().publish(
            (symbol_short!("grant"), grantee, resource_id),
            symbol_short!("success"),
        );
        Ok(())
    }

    /// Revoke access permission from an entity for a specific resource.
    ///
    /// Authorised callers (any one of):
    /// - The original grantor of the permission.
    /// - Any address holding the `Admin` role.
    /// - Any address holding the `PayerReviewer` role.
    ///
    /// # Arguments
    /// * `revoker`     - Must be the original grantor, an Admin, or a PayerReviewer.
    /// * `revokee`     - The address losing access.
    /// * `resource_id` - The identifier of the resource.
    pub fn revoke_access(env: Env, revoker: Address, revokee: Address, resource_id: String) -> Result<(), ContractError> {
        revoker.require_auth();

        // Determine whether the revoker has a privileged role that allows
        // revoking any permission (Admin or PayerReviewer).
        let has_privileged_role =
            Self::require_role(&env, &revoker, &Role::Admin).is_ok()
            || Self::require_role(&env, &revoker, &Role::PayerReviewer).is_ok();

        // Remove from grantee's access list
        let access_key = DataKey::AccessList(revokee.clone());
        let access_list: Vec<AccessPermission> = env
            .storage()
            .persistent()
            .get(&access_key)
            .unwrap_or(Vec::new(&env));

        let mut new_access_list: Vec<AccessPermission> = Vec::new(&env);
        let mut found = false;

        for i in 0..access_list.len() {
            if let Some(permission) = access_list.get(i) {
                if permission.resource_id == resource_id {
                    // Verify revoker is either the original grantor or holds a
                    // privileged role.
                    if permission.granted_by != revoker && !has_privileged_role {
                        return Err(ContractError::NotAuthorizedToRevoke);
                    }
                    found = true;
                    // Skip this permission (effectively removing it)
                } else {
                    new_access_list.push_back(permission);
                }
            }
        }

        if !found {
            return Err(ContractError::AccessPermissionNotFound);
        }

        env.storage()
            .persistent()
            .set(&access_key, &new_access_list);

        // Remove from resource's authorized parties
        let resource_key = DataKey::ResourceAccess(resource_id.clone());
        let authorized: Vec<Address> = env
            .storage()
            .persistent()
            .get(&resource_key)
            .unwrap_or(Vec::new(&env));

        let mut new_authorized: Vec<Address> = Vec::new(&env);
        for i in 0..authorized.len() {
            if let Some(addr) = authorized.get(i) {
                if addr != revokee {
                    new_authorized.push_back(addr);
                }
            }
        }
        env.storage()
            .persistent()
            .set(&resource_key, &new_authorized);

        env.events().publish(
            (symbol_short!("revoke"), revokee, resource_id),
            symbol_short!("success"),
        );
        Ok(())
    }

    /// Check if an entity has access to a specific resource
    ///
    /// # Arguments
    /// * `entity` - The address to check
    /// * `resource_id` - The identifier of the resource
    ///
    /// # Returns
    /// `true` if the entity has valid (non-expired) access, `false` otherwise
    pub fn check_access(env: Env, entity: Address, resource_id: String) -> bool {
        let access_key = DataKey::AccessList(entity);
        let access_list: Vec<AccessPermission> = env
            .storage()
            .persistent()
            .get(&access_key)
            .unwrap_or(Vec::new(&env));

        let current_time = env.ledger().timestamp();

        for i in 0..access_list.len() {
            if let Some(permission) = access_list.get(i) {
                if permission.resource_id == resource_id {
                    // Check if permission is expired
                    if permission.expires_at == 0 || permission.expires_at > current_time {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get all entities with access to a specific resource
    ///
    /// # Arguments
    /// * `resource_id` - The identifier of the resource
    ///
    /// # Returns
    /// A vector of addresses that have access to the resource
    pub fn get_authorized_parties(env: Env, resource_id: String) -> Vec<Address> {
        let resource_key = DataKey::ResourceAccess(resource_id);
        env.storage()
            .persistent()
            .get(&resource_key)
            .unwrap_or(Vec::new(&env))
    }

    /// Get entity details by wallet address
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the entity
    ///
    /// # Returns
    /// The EntityData for the given wallet address
    pub fn get_entity(env: Env, wallet: Address) -> Result<EntityData, ContractError> {
        let key = DataKey::Entity(wallet);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::EntityNotFound)
    }

    /// Get all access permissions for an entity
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the entity
    ///
    /// # Returns
    /// A vector of all access permissions granted to the entity
    pub fn get_entity_permissions(env: Env, wallet: Address) -> Vec<AccessPermission> {
        let access_key = DataKey::AccessList(wallet);
        env.storage()
            .persistent()
            .get(&access_key)
            .unwrap_or(Vec::new(&env))
    }

    /// Update entity metadata
    ///
    /// # Arguments
    /// * `wallet` - The wallet address of the entity
    /// * `metadata` - Updated metadata information
    pub fn update_entity(env: Env, wallet: Address, metadata: String) -> Result<(), ContractError> {
        wallet.require_auth();

        let key = DataKey::Entity(wallet.clone());
        let mut entity: EntityData = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::EntityNotFound)?;

        entity.metadata = metadata;
        env.storage().persistent().set(&key, &entity);

        env.events()
            .publish((symbol_short!("upd_ent"), wallet), symbol_short!("success"));
        Ok(())
    }

    /// Deactivate an entity.
    ///
    /// Requires the caller to hold the `Admin` role.
    ///
    /// # Arguments
    /// * `caller` - Must hold the `Admin` role.
    /// * `wallet` - The wallet address of the entity to deactivate.
    pub fn deactivate_entity(env: Env, caller: Address, wallet: Address) -> Result<(), ContractError> {
        caller.require_auth();
        Self::require_role(&env, &caller, &Role::Admin)
            .map_err(|_| ContractError::OnlyAdminCanDeactivate)?;

        let key = DataKey::Entity(wallet.clone());
        let mut entity: EntityData = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::EntityNotFound)?;

        entity.active = false;
        env.storage().persistent().set(&key, &entity);

        env.events()
            .publish((symbol_short!("deact"), wallet), symbol_short!("success"));
        Ok(())
    }

    /// Register or update a W3C DID for the provided address.
    /// Self-registration only: `address` must authorize this call.
    ///
    /// DID format must start with `did:`.
    pub fn register_did(env: Env, address: Address, did: Bytes) -> Result<(), ContractError> {
        address.require_auth();
        Self::validate_did(&did)?;

        let key = DataKey::Did(address.clone());
        let old_did: Option<Bytes> = env.storage().persistent().get(&key);
        let old_hash: Option<BytesN<32>> = old_did.map(|d| env.crypto().sha256(&d).into());
        let new_hash: BytesN<32> = env.crypto().sha256(&did).into();

        env.storage().persistent().set(&key, &did);
        env.events()
            .publish((symbol_short!("did_aud"), address), (old_hash, new_hash));
        Ok(())
    }

    /// Returns the DID registered for an address, if present.
    pub fn get_did(env: Env, address: Address) -> Option<Bytes> {
        env.storage().persistent().get(&DataKey::Did(address))
    }

    fn validate_did(did: &Bytes) -> Result<(), ContractError> {
        if did.len() < 4 {
            return Err(ContractError::InvalidDidFormat);
        }
        let d = did.get(0).unwrap_or_default();
        let i = did.get(1).unwrap_or_default();
        let d2 = did.get(2).unwrap_or_default();
        let colon = did.get(3).unwrap_or_default();
        if d != b'd' || i != b'i' || d2 != b'd' || colon != b':' {
            return Err(ContractError::InvalidDidFormat);
        }
        Ok(())
    }
}
