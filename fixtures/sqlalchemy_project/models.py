"""Uses BytesGeometry so the type class stays live."""

from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column

from geo_types import BytesGeometry


class Base(DeclarativeBase):
    pass


class Place(Base):
    __tablename__ = "places"

    id: Mapped[int] = mapped_column(primary_key=True)
    geom: Mapped[bytes | None] = mapped_column(BytesGeometry(srid=4326))
