"""Entry module for the greeter app — uses only a subset of core APIs."""

from libs.core.api import Greeter
from libs.core.util import USED_CONSTANT


def run() -> None:
    g = Greeter("world")
    print(g.greet())
    print("constant:", USED_CONSTANT)


if __name__ == "__main__":
    run()
