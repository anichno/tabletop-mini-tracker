.PHONY = run_on_pi

run_on_pi:
	@[ "${REMOTE_PI}" ] || ( echo -e ">> REMOTE_PI is not set\nexport REMOTE_PI=\"user@host\""; exit 1 )
	cross build --target=aarch64-unknown-linux-gnu
	scp ../../target/aarch64-unknown-linux-gnu/debug/imu-rpi-data-collect $(REMOTE_PI):/tmp/
	ssh -t $(REMOTE_PI) RUST_LOG=debug /tmp/imu-rpi-data-collect