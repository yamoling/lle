from lle import LLE, ObservationType, Action


def main():
    env = LLE.from_file("lvl6", ObservationType.LAYERED)
    env.reset()
    env.render("human")
    import time

    time.sleep(0.2)
    env.render("human")
    input()


if __name__ == "__main__":
    main()
