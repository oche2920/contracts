#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror, symbol_short, Address, Env, String,
};

mod test;

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized     = 2,
    Unauthorized       = 3,
    NotAProvider       = 4,
    RecordNotFound     = 5,
}

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Initialized,
    Admin,
    Provider(Address),
    Record(String),
    ProviderRecords(Address),
    ProviderRecordCount(Address),
    RateLimitConfig,
    ProviderRate(Address),
    ProviderReputation(Address),
    ProviderRatingByPatient(Address, Address),
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct ProviderRegistry;

#[contractimpl]
impl ProviderRegistry {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        Self::assert_not_initialized(&env)?;
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::Initialized, &true);
        Ok(())
    }

    pub fn register_provider(env: Env, admin: Address, provider: Address) -> Result<(), Error> {
        Self::assert_initialized(&env)?;
        Self::assert_admin(&env, &admin)?;
        env.storage()
            .persistent()
            .set(&DataKey::Provider(provider.clone()), &profile);
        env.events()
            .publish((symbol_short!("reg_prov"), provider), symbol_short!("ok"));
        Ok(())
    }

    pub fn revoke_provider(env: Env, admin: Address, provider: Address) -> Result<(), Error> {
        Self::assert_initialized(&env)?;
        Self::assert_admin(&env, &admin)?;
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::NotFound)?;

        profile.active = false;
        profile.credential.revoked_at = Some(env.ledger().timestamp());
        profile.credential.revoked_by = Some(admin.clone());
        env.storage().persistent().set(&key, &profile);

        env.events()
            .publish((symbol_short!("rev_prov"), provider), symbol_short!("ok"));
        Ok(())
    }

    pub fn is_provider(env: Env, provider: Address) -> bool {
        Self::provider_is_active(&env, &provider)
    }

    pub fn get_provider_profile(
        env: Env,
        provider: Address,
    ) -> Result<ProviderProfile, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Provider(provider))
            .ok_or(ContractError::NotFound)
    }

    pub fn add_record(
        env: Env,
        provider: Address,
        record_id: String,
        data: String,
    ) -> Result<(), Error> {
        Self::assert_initialized(&env)?;
        provider.require_auth();
        if !Self::is_provider(env.clone(), provider.clone()) {
            return Err(Error::NotAProvider);
        }
        env.storage()
            .persistent()
            .set(&DataKey::Record(record_id.clone()), &data);
        env.events()
            .publish((symbol_short!("add_rec"), provider, record_id), symbol_short!("ok"));
        Ok(())
    }

    pub fn get_record(env: Env, record_id: String) -> Result<String, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Record(record_id))
            .ok_or(Error::RecordNotFound)
    }

    // ── guards ────────────────────────────────────────────────────────────────

    fn assert_initialized(env: &Env) -> Result<(), Error> {
        if !env.storage().persistent().has(&DataKey::Initialized) {
            return Err(Error::NotInitialized);
        }
        Ok(())
    }

    fn assert_not_initialized(env: &Env) -> Result<(), Error> {
        if env.storage().persistent().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        Ok(())
    }

    fn assert_admin(env: &Env, caller: &Address) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if *caller != admin {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
}
