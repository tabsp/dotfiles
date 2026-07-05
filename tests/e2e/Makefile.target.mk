.PHONY: docker-e2e docker-e2e-build docker-e2e-no-network

docker-e2e:
	docker build -t dotman-e2e -f tests/e2e/Dockerfile .
	docker run --rm dotman-e2e

docker-e2e-build:
	docker build -t dotman-e2e -f tests/e2e/Dockerfile .

docker-e2e-no-network:
	docker build -t dotman-e2e -f tests/e2e/Dockerfile .
	docker run --rm -e SKIP_NETWORK_TESTS=1 dotman-e2e
