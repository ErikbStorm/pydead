"""Helpers that are never imported by main or anything else."""


def never_called() -> int:
    return 42


def also_never_called() -> str:
    return never_called()  # local use only; still dead from monorepo view if nothing imports module
    # Note: never_called is used here, so only also_never_called is dead at name level?
    # With name-based analysis, never_called is used (called above), also_never_called is not.
