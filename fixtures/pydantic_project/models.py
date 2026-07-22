"""Pydantic models with hooks that the library invokes (never called by app code)."""

from typing import Any, Annotated

from pydantic import (
    BaseModel,
    Field,
    field_validator,
    model_validator,
    field_serializer,
    model_serializer,
    computed_field,
    AfterValidator,
    BeforeValidator,
)
from pydantic_core import core_schema


def strip_spaces(v: str) -> str:
    """Used via AfterValidator(strip_spaces) — name reference keeps it live without EP006."""
    return v.strip()


def to_upper_before(v: Any) -> Any:
    """Used via BeforeValidator — live via name reference."""
    if isinstance(v, str):
        return v.upper()
    return v


class User(BaseModel):
    name: Annotated[str, AfterValidator(strip_spaces), BeforeValidator(to_upper_before)]
    age: int = Field(ge=0)
    tags: list[str] = Field(default_factory=list)

    @field_validator("name", mode="after")
    @classmethod
    def name_must_not_be_empty(cls, v: str) -> str:
        """EP006 — Pydantic calls this after parsing `name`."""
        if not v:
            raise ValueError("name empty")
        return v

    @field_validator("tags", mode="before")
    @classmethod
    def split_tags(cls, v: Any) -> Any:
        """EP006 — before validator."""
        if isinstance(v, str):
            return [p.strip() for p in v.split(",") if p.strip()]
        return v

    @model_validator(mode="after")
    def check_adult_name(self) -> "User":
        """EP006 — model after validator."""
        if self.age < 18 and self.name == "ADMIN":
            raise ValueError("invalid")
        return self

    @field_serializer("name")
    def serialize_name(self, v: str) -> str:
        """EP006 — field serializer."""
        return v.title()

    @model_serializer
    def ser_model(self) -> dict[str, Any]:
        """EP006 — model serializer."""
        return {"name": self.name, "age": self.age}

    @computed_field
    @property
    def display(self) -> str:
        """EP006 — computed field (decorator attr computed_field)."""
        return f"{self.name} ({self.age})"

    def model_post_init(self, __context: Any) -> None:
        """EP006 — Pydantic lifecycle hook."""
        self.tags = list(self.tags)

    def never_called_helper(self) -> str:
        """Should be reported dead (DC003)."""
        return "dead"


class FancyInt:
    """Custom type with Pydantic v2 schema hooks."""

    def __init__(self, value: int) -> None:
        self.value = value

    @classmethod
    def __get_pydantic_core_schema__(cls, source_type: Any, handler: Any) -> core_schema.CoreSchema:
        """EP006 — schema hook called by Pydantic."""
        return core_schema.no_info_after_validator_function(
            cls,
            core_schema.int_schema(),
        )

    @classmethod
    def __get_pydantic_json_schema__(cls, core_schema_: Any, handler: Any) -> dict[str, Any]:
        """EP006 — JSON schema hook."""
        json_schema = handler(core_schema_)
        json_schema.update(examples=[1, 2, 3])
        return json_schema


# v1-style names (still common in mixed codebases)
from pydantic.v1 import validator as v1_validator  # type: ignore
from pydantic.v1 import root_validator as v1_root_validator  # type: ignore
from pydantic.v1 import BaseModel as V1BaseModel  # type: ignore


class LegacyItem(V1BaseModel):
    sku: str
    qty: int

    @v1_validator("sku")
    def sku_upper(cls, v: str) -> str:
        """EP006 — v1 @validator."""
        return v.upper()

    @v1_root_validator
    def qty_positive(cls, values: dict) -> dict:
        """EP006 — v1 @root_validator."""
        if values.get("qty", 0) < 0:
            raise ValueError("qty")
        return values


def orphan_module_func() -> None:
    """Truly unused — DC001."""
    pass
