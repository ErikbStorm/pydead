"""App models / helpers used from migrations."""


def used_by_migration() -> str:
    """Referenced from upgrade() — must stay live."""
    return "ok"


def unused_model_helper() -> str:
    """Never referenced — dead."""
    return "dead"
