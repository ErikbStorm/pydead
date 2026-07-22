mod common;

#[test]
fn sqlalchemy_hooks_and_noqa() {
    common::assert_expected("sqlalchemy_project");
}
