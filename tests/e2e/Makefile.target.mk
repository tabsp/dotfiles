.PHONY: docker-e2e docker-e2e-build docker-e2e-no-network docker-e2e-sudo docker-e2e-sudo-build

docker-e2e:
	docker build -t dotman-e2e -f tests/e2e/Dockerfile .
	docker run --rm dotman-e2e

docker-e2e-build:
	docker build -t dotman-e2e -f tests/e2e/Dockerfile .

docker-e2e-no-network:
	docker build -t dotman-e2e -f tests/e2e/Dockerfile .
	docker run --rm -e SKIP_NETWORK_TESTS=1 dotman-e2e

# Interactive sudo-password test — requires a TTY, user must type password.
#   make docker-e2e-sudo-build
#   docker run --rm -it dotman-e2e-sudo tests/e2e/scenarios/sudo-prompt-tui.sh
docker-e2e-sudo-build:
	docker build -t dotman-e2e-sudo -f tests/e2e/Dockerfile.sudo .

docker-e2e-sudo: docker-e2e-sudo-build
	docker run --rm -it dotman-e2e-sudo tests/e2e/scenarios/sudo-prompt-tui.sh
