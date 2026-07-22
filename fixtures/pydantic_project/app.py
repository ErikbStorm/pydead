"""Application code that constructs models (classes stay live)."""

from models import User, FancyInt, LegacyItem


def main() -> None:
    u = User(name="  ada  ", age=30, tags="a,b")
    _ = u.model_dump()
    _ = FancyInt(1)
    _ = LegacyItem(sku="abc", qty=1)


if __name__ == "__main__":
    main()
