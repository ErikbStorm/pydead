"""Legacy plugin code that nothing imports anymore."""


class LegacyPlugin:
    """Dead class — no references in the monorepo."""

    def activate(self) -> None:
        print("legacy")

    def deactivate(self) -> None:
        print("bye")


def legacy_setup() -> LegacyPlugin:
    """Also dead — never imported."""
    return LegacyPlugin()
