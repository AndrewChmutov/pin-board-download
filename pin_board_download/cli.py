from __future__ import annotations

import inspect
from abc import ABC, abstractmethod
from collections.abc import Iterable
from pathlib import Path
from typing import Annotated, Any, ClassVar, Required, Self, get_type_hints, override

from typer import CallbackParam, Context, Option, Typer
from typer import rich_utils
from typer.core import TyperOption
from typer.main import get_command_from_info, get_group, get_group_from_info
from typer.models import CommandInfo, OptionInfo, ParamMeta
from typer.rich_utils import rich_format_help, _print_options_panel, _get_rich_console

from pin_board_download.logging import Logger

def from_context_or_raise[T](ctx: Context, param: CallbackParam, value: T | None) -> T:
    print(value)
    print(param.name)

class Cli(ABC):
    _registered: ClassVar[set[type[Self]]] = set()

    def __init_subclass__(cls) -> None:
        cls._init_cls_self()
        cls._init_cls_to_parents()

    @classmethod
    def _init_cls_self(cls) -> None:
        cls._registered = set()

    @classmethod
    def _init_cls_to_parents(cls) -> None:
        for parent_cls in cls.__bases__:
            parent_cls._registered.add(cls)

    @classmethod
    def register(
        cls, predecessors: tuple[type[Self], ...] = tuple(),
    ) -> Typer:
        encountered = predecessors + (cls,)
        Logger.debug(f"Register {cls._format_clss(encountered)}")

        app = cls._register_command_with_prevs(encountered)
        for registered_cls in cls._registered:
            cls._register_subcommand_rec(app, registered_cls, encountered)

        return app

    @classmethod
    def _register_subcommand_rec(
        cls,
        app: Typer,
        ccls: type[Self],
        encountered: tuple[type[Self], ...],
    ) -> None:
        app.add_typer(
            ccls.register(encountered),
            name=ccls.name()
        )

    @classmethod
    def _format_clss(cls, predecessors: Iterable[type[Self]]) -> str:
        return " > ".join(map(lambda x: x.__name__, predecessors))

    @classmethod
    def _register_command_with_prevs(cls, encountered: Iterable[type[Self]]) -> Typer:
        app = Typer()

        cmd_info = CommandInfo(name=cls.name(), callback=cls.cli)
        cmd = get_command_from_info(cmd_info, pretty_exceptions_short=True, rich_markup_mode="rich")

        def verify(ctx: Context, value: bool):
            if ctx.invoked_subcommand:
                Logger.debug("skipping")
                return False
            if value:
                group = get_group(app)
                group.help = None
                rich_format_help(obj=group, ctx=ctx, markup_mode="rich")
                _print_options_panel(
                    name="Options",
                    params=cmd.get_params(ctx),
                    ctx=ctx,
                    markup_mode="rich",
                    console=_get_rich_console(),
                )
                ctx.exit()
            return False


        def inner(
            ctx: Context,
            help: bool = Option(False, "--help", "-h", hidden=True, callback=verify),
        ) -> None:
            if ctx.invoked_subcommand or help:
                Logger.debug("skipping")
                return
            Logger.debug(f"{cls.name()}, {ctx.invoked_subcommand}")
            Logger.info(f"Calling {cls._format_clss(encountered)}")
            print("not finished board")
            print(ctx.args)
            cmd.parse_args(ctx, ctx.args)
            print("finished board")

        parameters = list(inspect.signature(inner).parameters.values())[:-1]
        # Logger.warning("Before", inspect.signature(inner).parameters)
        parameters.extend(cls._collect_encountered_parameters(encountered))
        Logger.warning(inspect.signature(inner).parameters)
        inner.__signature__ = inspect.Signature(parameters)
        # print(inner.__signature__)
        Logger.critical(inspect.signature(inner).parameters)

        app.callback(invoke_without_command=True, context_settings={"ignore_unknown_options": True})(inner)

        # cmd_info = CommandInfo(callback=main)
        # cmd.params = list(cls._collect_encountered_parameters(encountered))
        # app.registered_commands.append(cmd_info)
        return app

    @classmethod
    def _collect_encountered_parameters(cls, predecessors: Iterable[type[Self]]) -> Iterable[inspect.Parameter]:
        visited: dict[str, inspect.Parameter] = {}
        for predecessor in predecessors:
            for option in predecessor._cli_parameters():
                Logger.error(option)
                visited[option.name] = option
        yield from visited.values()


    @classmethod
    def _cli_parameters(cls) -> Iterable[inspect.Parameter]:
        yield from map(
            cls._cli_parameter_process,
            cls._cli_parameters_raw(),
        )

    @classmethod
    def _cli_parameters_raw(cls) -> Iterable[tuple[inspect.Parameter, Any]]:
        yield from zip(inspect.signature(cls.cli).parameters.values(), get_type_hints(cls.cli))

    @classmethod
    def _cli_parameter_process(cls, parameter: tuple[inspect.Parameter, get_type_i) -> inspect.Parameter:
        Logger.debug(f"{parameter.default}, {parameter.kind}, {parameter.name}, {type(parameter.annotation)}")
        empty = parameter.default is inspect._empty
        return inspect.Parameter(
            name=parameter.name,
            kind=parameter.kind,
            annotation=parameter.annotation,
            default=OptionInfo(default=parameter.default),
        )

    @classmethod
    def cli(cls) -> None:
        ...

    @classmethod
    @abstractmethod
    def name(cls) -> str:
        ...


class BoardCommand(Cli):
    @classmethod
    @override
    def cli(cls, url: int, board_page: Path) -> None:  # pyright: ignore[reportIncompatibleMethodOverride]
        print("Finally")

    @classmethod
    @override
    def name(cls) -> str:
        return "board"


class DownloadImageCommand(BoardCommand, Cli):
    @classmethod
    @override
    def cli(cls, board_page: Path, board: Path) -> None:  # pyright: ignore[reportIncompatibleMethodOverride]
        ...

    @classmethod
    @override
    def name(cls) -> str:
        return "download"

class Kik(DownloadImageCommand, Cli):
    @classmethod
    @override
    def cli(cls, board_page: Path, board: Path) -> None:  # pyright: ignore[reportIncompatibleMethodOverride]
        ...

    @classmethod
    @override
    def name(cls) -> str:
        return "kik"

app = Cli.register()
