import logging

import coloredlogs


class _Logger(logging.Logger):
    FMT = "%(asctime)s %(filename)s:%(lineno)d %(levelname)s %(message)s"  # noqa: E501

    def __init__(self, level: int | str = logging.NOTSET) -> None:
        assert __package__
        super().__init__(name=__package__)
        coloredlogs.install(logger=self, level=level, fmt=self.FMT)


Logger = _Logger()
