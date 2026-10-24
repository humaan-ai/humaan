#!/usr/bin/env python3
"""Stream SO-101 measured motor angles (degrees) as bridge-compatible JSON lines.

Run in the LeRobot environment used to calibrate the arm. This opens the motor
bus directly, avoiding robot.connect(), which configures motors and torque.
No calibration, torque changes, or position commands are issued.
"""
import argparse
import contextlib
import json
import math
from pathlib import Path
import sys
import time


def stream(robot, fps, output):
    bus = robot.bus
    if not robot.calibration:
        raise RuntimeError(
            f"No saved calibration found at {robot.calibration_fpath}. "
            "Use the same environment and robot ID used during calibration, "
            "or supply --calibration-dir."
        )
    try:
        bus.connect()
        if not bus.is_calibrated:
            raise RuntimeError("Saved calibration does not match the motors; refusing to stream.")
        while True:
            started = time.monotonic()
            positions = {name: float(value) for name, value in bus.sync_read("Present_Position").items()}
            if not positions or not all(math.isfinite(value) for value in positions.values()):
                raise RuntimeError("Invalid measured joint positions.")
            print(json.dumps(positions, allow_nan=False), file=output, flush=True)
            time.sleep(max(0, 1 / fps - (time.monotonic() - started)))
    finally:
        if bus.is_connected:
            bus.disconnect(disable_torque=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", required=True, help="USB serial device path")
    parser.add_argument("--id", required=True, help="Existing LeRobot calibration ID")
    parser.add_argument("--calibration-dir", type=Path)
    parser.add_argument("--fps", type=float, default=20)
    args = parser.parse_args()
    if not math.isfinite(args.fps) or not 0 < args.fps <= 100:
        parser.error("--fps must be greater than 0 and at most 100")
    output = sys.stdout
    try:
        # Keep dependency diagnostics out of the JSON stream.
        with contextlib.redirect_stdout(sys.stderr):
            try:
                from lerobot.robots.so_follower import SO101Follower, SO101FollowerConfig
            except ModuleNotFoundError as error:
                if error.name != "lerobot.robots.so_follower":
                    raise
                from lerobot.robots.so101_follower import SO101Follower, SO101FollowerConfig
            from lerobot.motors import MotorNormMode

            config = SO101FollowerConfig(
                port=args.port, id=args.id, calibration_dir=args.calibration_dir,
                use_degrees=True,
            )
            robot = SO101Follower(config)
            # LeRobot normally exposes gripper as 0–100%, which is not an angle.
            # Read its servo angle too; URDF direction/zero must be mapped in Studio.
            for motor in robot.bus.motors.values():
                motor.norm_mode = MotorNormMode.DEGREES
            stream(robot, args.fps, output)
    except KeyboardInterrupt:
        return 0
    except BrokenPipeError:
        return 0
    except Exception as error:
        print(f"SO-101 telemetry stopped: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
