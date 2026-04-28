#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::Env;

#[test]
fn test_add_and_get_records() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);
    let ipfs_hash = String::from_str(&e, "QmXoypizj2Madv6NthR75ce451F33968F9e1XF3D8xS288");
    let description = String::from_str(&e, "Test Tax Document");

    client.add_financial_record(&owner, &RecordType::TaxDocument, &ipfs_hash, &description);

    let records = client.get_financial_records(&owner, &owner, &0, &10);
    assert_eq!(records.len(), 1);
    let record = records.get(0).unwrap();
    assert_eq!(record.owner, owner);
    assert_eq!(record.record_type, RecordType::TaxDocument);
    assert_eq!(record.ipfs_hash, ipfs_hash);
    assert_eq!(record.description, description);
}

#[test]
fn test_access_granted() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);
    let auditor = Address::generate(&e);

    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "hash"),
        &String::from_str(&e, "desc"),
    );

    client.grant_access(&owner, &auditor);

    let records = client.get_financial_records(&auditor, &owner, &0, &10);
    assert_eq!(records.len(), 1);
}

#[test]
fn test_unauthorized_access_returns_typed_error() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);
    let stranger = Address::generate(&e);

    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "h"),
        &String::from_str(&e, "d"),
    );

    let result = client.try_get_financial_records(&stranger, &owner, &0, &10);
    assert_eq!(result, Err(Ok(ContractError::AccessDenied)));
}

#[test]
fn test_revoked_access_returns_typed_error() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);
    let auditor = Address::generate(&e);

    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "h"),
        &String::from_str(&e, "d"),
    );

    client.grant_access(&owner, &auditor);
    let records = client.get_financial_records(&auditor, &owner, &0, &10);
    assert_eq!(records.len(), 1);

    client.revoke_access(&owner, &auditor);
    let result = client.try_get_financial_records(&auditor, &owner, &0, &10);
    assert_eq!(result, Err(Ok(ContractError::AccessDenied)));
}

#[test]
fn test_type_index_filtering() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);

    e.ledger().set_timestamp(100);
    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "h1"),
        &String::from_str(&e, "d1"),
    );

    e.ledger().set_timestamp(200);
    client.add_financial_record(
        &owner,
        &RecordType::TaxDocument,
        &String::from_str(&e, "h2"),
        &String::from_str(&e, "d2"),
    );

    e.ledger().set_timestamp(300);
    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "h3"),
        &String::from_str(&e, "d3"),
    );

    let invoices = client.get_records_by_type(&owner, &owner, &RecordType::Invoice, &0, &10);
    assert_eq!(invoices.len(), 2);

    let tax_docs = client.get_records_by_type(&owner, &owner, &RecordType::TaxDocument, &0, &10);
    assert_eq!(tax_docs.len(), 1);
}

#[test]
fn test_date_index_filtering() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);

    e.ledger().set_timestamp(100);
    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "h1"),
        &String::from_str(&e, "d1"),
    );

    e.ledger().set_timestamp(200);
    client.add_financial_record(
        &owner,
        &RecordType::TaxDocument,
        &String::from_str(&e, "h2"),
        &String::from_str(&e, "d2"),
    );

    e.ledger().set_timestamp(300);
    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "h3"),
        &String::from_str(&e, "d3"),
    );

    let range = client.get_records_by_date_range(&owner, &owner, &150, &250, &0, &10);
    assert_eq!(range.len(), 1);
    assert_eq!(range.get(0).unwrap().timestamp, 200);
}

#[test]
fn test_pagination_get_financial_records() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);

    for i in 0u32..5 {
        client.add_financial_record(
            &owner,
            &RecordType::Receipt,
            &String::from_str(&e, "h"),
            &String::from_str(&e, "d"),
        );
        let _ = i;
    }

    let page1 = client.get_financial_records(&owner, &owner, &0, &3);
    assert_eq!(page1.len(), 3);

    let page2 = client.get_financial_records(&owner, &owner, &3, &3);
    assert_eq!(page2.len(), 2);
}

#[test]
fn test_pagination_get_records_by_type() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);

    for _ in 0..4 {
        client.add_financial_record(
            &owner,
            &RecordType::Receipt,
            &String::from_str(&e, "h"),
            &String::from_str(&e, "d"),
        );
    }

    let page1 = client.get_records_by_type(&owner, &owner, &RecordType::Receipt, &0, &2);
    assert_eq!(page1.len(), 2);

    let page2 = client.get_records_by_type(&owner, &owner, &RecordType::Receipt, &2, &2);
    assert_eq!(page2.len(), 2);
}

#[test]
fn test_type_index_unauthorized_returns_typed_error() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);
    let stranger = Address::generate(&e);

    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "h"),
        &String::from_str(&e, "d"),
    );

    let result = client.try_get_records_by_type(&stranger, &owner, &RecordType::Invoice, &0, &10);
    assert_eq!(result, Err(Ok(ContractError::AccessDenied)));
}

#[test]
fn test_date_index_unauthorized_returns_typed_error() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(FinancialRecordContract, ());
    let client = FinancialRecordContractClient::new(&e, &contract_id);

    let owner = Address::generate(&e);
    let stranger = Address::generate(&e);

    client.add_financial_record(
        &owner,
        &RecordType::Invoice,
        &String::from_str(&e, "h"),
        &String::from_str(&e, "d"),
    );

    let result = client.try_get_records_by_date_range(&stranger, &owner, &0, &999, &0, &10);
    assert_eq!(result, Err(Ok(ContractError::AccessDenied)));
}
