use soroban_sdk::{contracttype, Env, Vec};

/// Maximum number of IDs stored in a single on-chain page Vec.
/// Bounds the ledger read/write cost of any single list operation.
pub const MAX_PAGE_SIZE: u32 = 20;

/// Sentinel value for `PageResult::next_page` indicating no further pages exist.
pub const NO_NEXT_PAGE: u32 = u32::MAX;

/// Returned by all paged list query functions.
///
/// Callers advance through pages by passing `next_page` back as the `page`
/// argument until `next_page == NO_NEXT_PAGE`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageResult {
    /// IDs on the requested page (length ≤ MAX_PAGE_SIZE).
    pub ids: Vec<u64>,
    /// Index of the next page, or `NO_NEXT_PAGE` when this is the last page.
    pub next_page: u32,
    /// Total number of items across all pages tracked by the calling contract.
    pub total: u32,
}

/// Push `id` into a paged list stored at keys produced by `page_key_fn` and
/// `head_key_fn`.
///
/// Contracts call this with closures that map `(Address, u32)` → their own
/// `DataKey` variant.  The helper reads the current head page, appends the
/// item, and—once the page is full—advances the head pointer so the next push
/// starts a fresh page.
///
/// # Storage layout
/// * `page_key_fn(page_num)` → `Vec<u64>` with ≤ `MAX_PAGE_SIZE` items
/// * `head_key_fn()`         → `u32` current page index (default: 0)
pub fn push_paged<FK, FH, K>(env: &Env, page_key_fn: FK, head_key_fn: FH, id: u64)
where
    FK: Fn(u32) -> K,
    FH: Fn() -> K,
    K: soroban_sdk::IntoVal<Env, soroban_sdk::Val>
        + soroban_sdk::TryFromVal<Env, soroban_sdk::Val>,
{
    let head_key = head_key_fn();
    let current_page: u32 = env
        .storage()
        .persistent()
        .get(&head_key)
        .unwrap_or(0u32);

    let page_key = page_key_fn(current_page);
    let mut page: Vec<u64> = env
        .storage()
        .persistent()
        .get(&page_key)
        .unwrap_or(Vec::new(env));

    page.push_back(id);
    env.storage().persistent().set(&page_key, &page);

    if page.len() as u32 >= MAX_PAGE_SIZE {
        env.storage()
            .persistent()
            .set(&head_key, &(current_page + 1));
    }
}

/// Read one page from a paged list and return a `PageResult`.
///
/// `head_key_fn` yields the key that stores the current (highest-written) page
/// index.  `page_key_fn(n)` yields the key for page `n`.  `total_fn` is an
/// optional closure that returns the total item count; pass `|| 0` to omit it.
pub fn get_paged<FK, FH, FT, K>(
    env: &Env,
    page_key_fn: FK,
    head_key_fn: FH,
    total_fn: FT,
    page: u32,
) -> PageResult
where
    FK: Fn(u32) -> K,
    FH: Fn() -> K,
    FT: Fn() -> u32,
    K: soroban_sdk::IntoVal<Env, soroban_sdk::Val>
        + soroban_sdk::TryFromVal<Env, soroban_sdk::Val>,
{
    let head_key = head_key_fn();
    let head: u32 = env
        .storage()
        .persistent()
        .get(&head_key)
        .unwrap_or(0u32);

    let page_key = page_key_fn(page);
    let ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&page_key)
        .unwrap_or(Vec::new(env));

    let is_last = page >= head && (ids.len() as u32) < MAX_PAGE_SIZE;
    let next_page = if is_last { NO_NEXT_PAGE } else { page + 1 };

    PageResult {
        ids,
        next_page,
        total: total_fn(),
    }
}
