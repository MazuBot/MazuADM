import logging
import os

from connector import run_connector


def configure_logging():
    level_name = os.getenv('LOG_LEVEL', 'INFO').upper()
    level = getattr(logging, level_name, logging.INFO)
    logging.basicConfig(
        level=level,
        format='%(asctime)s %(levelname)s [%(name)s] %(message)s',
    )


if __name__ == '__main__':
    configure_logging()
    run_connector()
