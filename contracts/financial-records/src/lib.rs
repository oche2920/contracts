#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, vec, Address, Env, String, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    AccessDenied = 1,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordType {
    TaxDocument = 0,
    Invoice = 1,
    Receipt = 2,
    BankStatement = 3,
    Other = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinancialRecord {
    pub owner: Address,
    pub record_type: RecordType,
    pub ipfs_hash: String,
    pub timestamp: u64,
    pub description: String,
}

#[contracttype]
pub enum DataKey {
    Record(Address, u32),     // (Owner, Index) -> FinancialRecord
    RecordCount(Address),     // Owner -> Number of records
    Access(Address, Address), // (Owner, Authorized) -> bool
}

#[contract]
pub struct FinancialRecordContract;

#[contractimpl]
impl FinancialRecordContract {
    /// Adds a new financial record for the caller.
    pub fn add_financial_record(
        e: Env,
        owner: Address,
        record_type: RecordType,
        ipfs_hash: String,
        description: String,
    ) {
        owner.require_auth();

        let count: u32 = e
            .storage()
            .persistent()
            .get(&DataKey::RecordCount(owner.clone()))
            .unwrap_or(0);
        let timestamp = e.ledger().timestamp();

        let record = FinancialRecord {
            owner: owner.clone(),
            record_type,
            ipfs_hash,
            timestamp,
            description,
        };

        e.storage()
            .persistent()
            .set(&DataKey::Record(owner.clone(), count), &record);
        e.storage()
            .persistent()
            .set(&DataKey::RecordCount(owner.clone()), &(count + 1));
    }

    /// Retrieves all financial records for an owner.
    /// Access is allowed if the caller is the owner or has been granted access.
    pub fn get_financial_records(e: Env, caller: Address, owner: Address) -> Result<Vec<FinancialRecord>, ContractError> {
        Self::check_access(&e, &caller, &owner)?;

        let count: u32 = e
            .storage()
            .persistent()
            .get(&DataKey::RecordCount(owner.clone()))
            .unwrap_or(0);
        let mut records = vec![&e];

        for i in 0..count {
            if let Some(record) = e
                .storage()
                .persistent()
                .get(&DataKey::Record(owner.clone(), i))
            {
                records.push_back(record);
            }
        }
        Ok(records)
    }

    /// Retrieves records within a specific date range.
    pub fn get_records_by_date_range(
        e: Env,
        caller: Address,
        owner: Address,
        start: u64,
        end: u64,
    ) -> Result<Vec<FinancialRecord>, ContractError> {
        Self::check_access(&e, &caller, &owner)?;

        let count: u32 = e
            .storage()
            .persistent()
            .get(&DataKey::RecordCount(owner.clone()))
            .unwrap_or(0);
        let mut records = vec![&e];

        for i in 0..count {
            if let Some(record) = e
                .storage()
                .persistent()
                .get::<DataKey, FinancialRecord>(&DataKey::Record(owner.clone(), i))
            {
                if record.timestamp >= start && record.timestamp <= end {
                    records.push_back(record);
                }
            }
        }
        Ok(records)
    }

    /// Retrieves records of a specific type.
    pub fn get_records_by_type(
        e: Env,
        caller: Address,
        owner: Address,
        record_type: RecordType,
    ) -> Result<Vec<FinancialRecord>, ContractError> {
        Self::check_access(&e, &caller, &owner)?;

        let count: u32 = e
            .storage()
            .persistent()
            .get(&DataKey::RecordCount(owner.clone()))
            .unwrap_or(0);
        let mut records = vec![&e];

        for i in 0..count {
            if let Some(record) = e
                .storage()
                .persistent()
                .get::<DataKey, FinancialRecord>(&DataKey::Record(owner.clone(), i))
            {
                if record.record_type == record_type {
                    records.push_back(record);
                }
            }
        }
        Ok(records)
    }

    /// Grants access to another address.
    pub fn grant_access(e: Env, owner: Address, authorized: Address) {
        owner.require_auth();
        e.storage()
            .persistent()
            .set(&DataKey::Access(owner, authorized), &true);
    }

    /// Revokes access from another address.
    pub fn revoke_access(e: Env, owner: Address, authorized: Address) {
        owner.require_auth();
        e.storage()
            .persistent()
            .remove(&DataKey::Access(owner, authorized));
    }

    /// Internal helper to check access.
    fn check_access(e: &Env, caller: &Address, owner: &Address) -> Result<(), ContractError> {
        if caller == owner {
            return Ok(());
        }
        let is_authorized: bool = e
            .storage()
            .persistent()
            .get(&DataKey::Access(owner.clone(), caller.clone()))
            .unwrap_or(false);

        if !is_authorized {
            return Err(ContractError::AccessDenied);
        }
        Ok(())
    }
}

mod test;
