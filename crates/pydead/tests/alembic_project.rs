mod common;

#[test]
fn alembic_upgrade_downgrade_are_live() {
    common::assert_expected("alembic_project");
}
