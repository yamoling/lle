import time

import cv2

import lle

MAP_FILE = "poc.toml"
WAIT_SECONDS = 20


def main():
    env = lle.from_file(MAP_FILE).build()
    env.reset()
    env.render()  # MARLEnv.render: cv2 window named after the env
    # Pump the cv2 event loop instead of time.sleep so the window stays drawn and responsive.
    deadline = time.monotonic() + WAIT_SECONDS
    while time.monotonic() < deadline:
        cv2.waitKey(100)
    cv2.destroyAllWindows()


if __name__ == "__main__":
    main()
