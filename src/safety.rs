/// Returns true only when the operator entered the exact resource name.
pub fn confirms_resource_name(expected: &str, entered: &str) -> bool {
    !expected.is_empty() && expected == entered
}

#[cfg(test)]
mod tests {
    use super::confirms_resource_name;

    #[test]
    fn destructive_confirmation_requires_an_exact_name() {
        assert!(confirms_resource_name("ORDERS", "ORDERS"));
        assert!(!confirms_resource_name("ORDERS", "orders"));
        assert!(!confirms_resource_name("ORDERS", "ORDERS "));
        assert!(!confirms_resource_name("ORDERS", "ORDERS-OTHER"));
        assert!(!confirms_resource_name("", ""));
    }
}
