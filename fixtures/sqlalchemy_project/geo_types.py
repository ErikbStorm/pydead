"""SQLAlchemy TypeDecorator hooks — called by the engine, not by app code."""

from __future__ import annotations

from typing import Any

from sqlalchemy import Text, event, text as sa_text
from sqlalchemy.engine import Dialect
from sqlalchemy.orm import validates
from sqlalchemy.sql import BindParameter
from sqlalchemy.types import TypeDecorator, TypeEngine


class Geometry(TypeEngine[Any]):
    """Marker geometry type for dialect DDL."""

    cache_ok = True


class BytesGeometry(TypeDecorator[bytes | None]):
    """
    Type Decorator for binding WKB bytes to a sql type.
    """

    impl = Geometry
    cache_ok = True

    def __init__(
        self,
        srid: int = 4326,  # Default to WGS84, common for geography
    ) -> None:
        super().__init__()
        self.srid: int = srid

    def load_dialect_impl(self, dialect: Dialect) -> Geometry:
        # Tell SQLAlchemy we're using our marker type for DDL
        return Geometry()

    def column_expression(self, col):  # noqa: ANN001
        """Select rewrite — SQLAlchemy entry (EP007)."""
        col_str = str(col)
        return sa_text(f"{col_str}.STAsBinary()")

    def bind_expression(self, bindvalue: BindParameter[bytes]):
        """Insert/update rewrite — SQLAlchemy entry (EP007)."""
        return sa_text(
            f"geometry::STGeomFromWKB(:{bindvalue.key},{self.srid})"
        ).bindparams(bindvalue)

    def process_bind_param(self, value: bytes | None, dialect: Dialect) -> bytes | None:
        return value

    def process_result_value(self, value: bytes | None, dialect: Dialect) -> bytes | None:
        return value

    def never_used_method(self) -> str:
        """Should be dead (DC003)."""
        return "dead"


class UserRow:
    email: str

    @validates("email")
    def validate_email(self, key: str, address: str) -> str:
        """ORM validates hook — EP007."""
        return address.lower()

    def other_unused(self) -> None:
        """Dead."""
        pass


def unused_helper() -> None:
    """Dead module function."""
    pass


# Line-ignore demo (would be DC001 without noqa)
def intentionally_kept_local() -> int:  # pydead: keep
    return 1


# Explicit keep on a method that would otherwise be DC003
class KeepDemo:
    def plugin_hook(self) -> str:  # pydead: keep
        """Framework loads this by string — mark keep so it is not unused."""
        return "ok"
