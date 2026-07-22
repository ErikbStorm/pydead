mod common;

#[test]
fn azure_functions_v2_entry_points_are_live() {
    common::assert_expected("azure_functions_v2");
}
