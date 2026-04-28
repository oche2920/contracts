#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, xdr::ToXdr, Address, Bytes,
    BytesN, Env, String, Vec,
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
    // #228: commit-reveal
    CommitNotFound = 12,
    CommitHashMismatch = 13,
    CommitAlreadyUsed = 14,
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
/// #222: op_id added for correlation / forensic auditability
/// --------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessPermission {
    pub resource_id: String,
    pub granted_by: Address,
    pub granted_at: u64,
    pub expires_at: u64, // 0 means no expiration
    pub op_id: u64,      // #222: immutable operation receipt / correlation ID
}

/// --------------------
/// #228: Pending commit for commit-reveal anti-front-running
/// --------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingCommit {
    pub committer: Address,
    pub committed_at: u64,
    pub used: bool,
}

/// --------------------
/// Storage Keys
/// --------------------
#[contracttype]
pub enum DataKey {
    Admin,
    Entity(Address),
    AccessList(Address),                    // Entity -> Vec<AccessPermission>
    ResourceAccess(String),                 // Resource -> Vec<Address> (authorized parties)
    Did(Address),
    // #220: composite uniqueness index: (grantor, grantee, resource) -> bool
    GrantIndex(Address, Address, String),
    // #222: monotonic operation counter
    OpCounter,
    // #228: commit-reveal: hash -> PendingCommit
    Commit(BytesN<32>),
}

#[contract]
pub struct AccessControl;

#[contractimpl]
impl AccessControl {
    /// Initialize the contract with an admin
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().persistent().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &admin);

        env.events()
            .publish((symbol_short!("init"), admin), symbol_short!("success"));
        Ok(())
    }

    /// Register a new entity in the system
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

    // -----------------------------------------------------------------------
    // #228: Commit phase — caller submits hash(nonce || grantor || grantee ||
    //       resource_id) before the reveal (grant_access) call.
    // -----------------------------------------------------------------------
    /// Commit a hash before calling grant_access to prevent front-running.
    ///
    /// # Arguments
    /// * `committer` - The address that will later call grant_access
    /// * `commit_hash` - sha256(nonce || grantor || grantee || resource_id)
    pub fn commit_grant(
        env: Env,
        committer: Address,
        commit_hash: BytesN<32>,
    ) -> Result<(), ContractError> {
        committer.require_auth();

        let key = DataKey::Commit(commit_hash.clone());
        // Reject re-use of the same hash
        if env.storage().temporary().has(&key) {
            return Err(ContractError::CommitAlreadyUsed);
        }

        let commit = PendingCommit {
            committer: committer.clone(),
            committed_at: env.ledger().timestamp(),
            used: false,
        };
        // Store with a TTL of ~1 hour (3600 ledgers at ~1s each)
        env.storage().temporary().set(&key, &commit);
        env.storage()
            .temporary()
            .extend_ttl(&key, 3600, 3600);

        env.events()
            .publish((symbol_short!("committed"), committer), commit_hash);
        Ok(())
    }

    /// Grant access permission to an entity for a specific resource.
    ///
    /// For sensitive operations, callers should first call `commit_grant` with
    /// hash(nonce || grantor || grantee || resource_id) and pass the same
    /// `nonce` here so the contract can verify the commit (anti-front-running).
    ///
    /// Pass `nonce = None` to skip commit-reveal verification (backward-compat).
    ///
    /// # Arguments
    /// * `grantor`      - The address granting access (must be authorized)
    /// * `grantee`      - The address receiving access
    /// * `resource_id`  - The identifier of the resource
    /// * `expires_at`   - Expiration timestamp (0 for no expiration)
    /// * `nonce`        - Optional nonce used in commit_grant
    pub fn grant_access(
        env: Env,
        grantor: Address,
        grantee: Address,
        resource_id: String,
        expires_at: u64,
        nonce: Option<BytesN<32>>,
    ) -> Result<u64, ContractError> {
        grantor.require_auth();

        // #228: verify commit if nonce provided
        if let Some(n) = nonce {
            Self::verify_and_consume_commit(&env, &grantor, &grantee, &resource_id, n)?;
        }

        // Verify grantor is a registered entity
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Entity(grantor.clone()))
        {
            return Err(ContractError::GrantorNotRegistered);
        }

        // Verify grantee is a registered entity
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Entity(grantee.clone()))
        {
            return Err(ContractError::GranteeNotRegistered);
        }

        // #220: composite uniqueness check — (grantor, grantee, resource_id)
        let grant_idx = DataKey::GrantIndex(
            grantor.clone(),
            grantee.clone(),
            resource_id.clone(),
        );
        if env.storage().persistent().has(&grant_idx) {
            return Err(ContractError::AccessAlreadyGranted);
        }

        // #222: assign monotonic operation ID
        let op_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::OpCounter)
            .unwrap_or(0u64)
            + 1;
        env.storage().instance().set(&DataKey::OpCounter, &op_id);

        let permission = AccessPermission {
            resource_id: resource_id.clone(),
            granted_by: grantor.clone(),
            granted_at: env.ledger().timestamp(),
            expires_at,
            op_id,
        };

        // Add permission to grantee's access list
        let access_key = DataKey::AccessList(grantee.clone());
        let mut access_list: Vec<AccessPermission> = env
            .storage()
            .persistent()
            .get(&access_key)
            .unwrap_or(Vec::new(&env));

        access_list.push_back(permission);
        env.storage().persistent().set(&access_key, &access_list);

        // #220: record composite grant index
        env.storage().persistent().set(&grant_idx, &true);

        // #224: add grantee to resource's authorized parties (symmetric index)
        let resource_key = DataKey::ResourceAccess(resource_id.clone());
        let mut authorized: Vec<Address> = env
            .storage()
            .persistent()
            .get(&resource_key)
            .unwrap_or(Vec::new(&env));
        authorized.push_back(grantee.clone());
        env.storage().persistent().set(&resource_key, &authorized);

        // #222: include op_id in event for correlation
        env.events().publish(
            (symbol_short!("grant"), grantee, resource_id),
            op_id,
        );
        Ok(op_id)
    }

    /// Revoke access permission from an entity for a specific resource.
    ///
    /// Atomically removes the permission from ALL indexes:
    ///   1. grantee's AccessList
    ///   2. resource's ResourceAccess list
    ///   3. composite GrantIndex
    ///
    /// # Arguments
    /// * `revoker`     - The address revoking access (must be the original grantor or admin)
    /// * `revokee`     - The address losing access
    /// * `resource_id` - The identifier of the resource
    pub fn revoke_access(
        env: Env,
        revoker: Address,
        revokee: Address,
        resource_id: String,
    ) -> Result<u64, ContractError> {
        revoker.require_auth();

        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(ContractError::ContractNotInitialized)?;

        // --- Step 1: remove from grantee's access list, capture grantor ---
        let access_key = DataKey::AccessList(revokee.clone());
        let access_list: Vec<AccessPermission> = env
            .storage()
            .persistent()
            .get(&access_key)
            .unwrap_or(Vec::new(&env));

        let mut new_access_list: Vec<AccessPermission> = Vec::new(&env);
        let mut found_grantor: Option<Address> = None;
        let mut revoked_op_id: u64 = 0;

        for i in 0..access_list.len() {
            if let Some(permission) = access_list.get(i) {
                if permission.resource_id == resource_id && found_grantor.is_none() {
                    // Verify revoker is either the original grantor or admin
                    if permission.granted_by != revoker && revoker != admin {
                        return Err(ContractError::NotAuthorizedToRevoke);
                    }
                    found_grantor = Some(permission.granted_by.clone());
                    revoked_op_id = permission.op_id;
                    // skip — effectively removing it
                } else {
                    new_access_list.push_back(permission);
                }
            }
        }

        let grantor = found_grantor.ok_or(ContractError::AccessPermissionNotFound)?;

        env.storage()
            .persistent()
            .set(&access_key, &new_access_list);

        // --- Step 2: remove from resource's authorized parties (#224 atomic) ---
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

        // --- Step 3: remove composite grant index (#220 + #224 symmetric) ---
        let grant_idx = DataKey::GrantIndex(
            grantor,
            revokee.clone(),
            resource_id.clone(),
        );
        env.storage().persistent().remove(&grant_idx);

        // #222: assign op_id for the revocation event
        let op_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::OpCounter)
            .unwrap_or(0u64)
            + 1;
        env.storage().instance().set(&DataKey::OpCounter, &op_id);

        // #222: include both the revocation op_id and the original grant op_id
        env.events().publish(
            (symbol_short!("revoke"), revokee, resource_id),
            (op_id, revoked_op_id),
        );
        Ok(op_id)
    }

    /// Check if an entity has access to a specific resource
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
                    if permission.expires_at == 0 || permission.expires_at > current_time {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get all entities with access to a specific resource
    pub fn get_authorized_parties(env: Env, resource_id: String) -> Vec<Address> {
        let resource_key = DataKey::ResourceAccess(resource_id);
        env.storage()
            .persistent()
            .get(&resource_key)
            .unwrap_or(Vec::new(&env))
    }

    /// Get entity details by wallet address
    pub fn get_entity(env: Env, wallet: Address) -> Result<EntityData, ContractError> {
        let key = DataKey::Entity(wallet);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::EntityNotFound)
    }

    /// Get all access permissions for an entity
    pub fn get_entity_permissions(env: Env, wallet: Address) -> Vec<AccessPermission> {
        let access_key = DataKey::AccessList(wallet);
        env.storage()
            .persistent()
            .get(&access_key)
            .unwrap_or(Vec::new(&env))
    }

    /// Update entity metadata
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

    /// Deactivate an entity (admin only)
    pub fn deactivate_entity(
        env: Env,
        admin: Address,
        wallet: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(ContractError::ContractNotInitialized)?;

        if admin != stored_admin {
            return Err(ContractError::OnlyAdminCanDeactivate);
        }

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

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

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

    /// #228: Verify that a valid commit exists for (grantor, grantee, resource_id, nonce)
    /// and mark it as used.
    fn verify_and_consume_commit(
        env: &Env,
        grantor: &Address,
        grantee: &Address,
        resource_id: &String,
        nonce: BytesN<32>,
    ) -> Result<(), ContractError> {
        // Reconstruct the expected hash: sha256(nonce || grantor_xdr || grantee_xdr || resource_xdr)
        let mut data = Bytes::new(env);
        data.append(&nonce.clone().into());
        data.append(&grantor.clone().to_xdr(env));
        data.append(&grantee.clone().to_xdr(env));
        data.append(&resource_id.clone().to_xdr(env));
        let expected_hash: BytesN<32> = env.crypto().sha256(&data).into();

        let key = DataKey::Commit(expected_hash.clone());
        let mut commit: PendingCommit = env
            .storage()
            .temporary()
            .get(&key)
            .ok_or(ContractError::CommitNotFound)?;

        if commit.used {
            return Err(ContractError::CommitAlreadyUsed);
        }

        commit.used = true;
        env.storage().temporary().set(&key, &commit);
        Ok(())
    }
}
