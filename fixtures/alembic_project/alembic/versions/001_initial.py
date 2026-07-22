"""Initial revision — Alembic calls upgrade/downgrade by name."""

from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa

from app.models import used_by_migration

revision: str = "001"
down_revision: Union[str, None] = None
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    """Alembic entry point (EP005) — must stay live."""
    op.create_table(
        "items",
        sa.Column("id", sa.Integer(), primary_key=True),
        sa.Column("name", sa.String(length=64), nullable=False),
    )
    used_by_migration()


def downgrade() -> None:
    """Alembic entry point (EP005) — must stay live."""
    op.drop_table("items")


def leftover_helper() -> None:
    """Not called by Alembic or app — should be DC001 dead."""
    pass
