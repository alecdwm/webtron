################################################################################
SHELL := /bin/bash

WEBTRONPATH = $(GOPATH)/src/go.owls.io/webtron

phony:
	@echo -ne "\033[0;33mAvailable commands:\033[0m\n\n\
make test\t- removes, builds then runs the server executable\n\
\n\
make build\t- builds the server executable\n\
make run\t- runs the server executable\n\
make clean\t- deletes the server executable\n"

################################################################################
build:
	@echo "Building webtron"
	@go build go.owls.io/webtron

run:
ifneq "$(PORT)" ""
	@if [ -f $(WEBTRONPATH)/webtron ]; then \
	echo "Running webtron on port $(PORT)"; \
	$(WEBTRONPATH)/webtron -listenPort $(PORT); fi
else
	@if [ -f $(WEBTRONPATH)/webtron ]; then \
	echo "Running webtron on default port"; \
	$(WEBTRONPATH)/webtron; fi
endif

clean:
	@if [ -f $(WEBTRONPATH)/webtron ]; then \
	echo "Removing webtron"; \
	rm -f $(WEBTRONPATH)/webtron; fi

test: clean build run
