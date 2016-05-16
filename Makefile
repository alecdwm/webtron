################################################################################
SHELL := /bin/bash

WEBTRONPATH = $(GOPATH)/src/go.owls.io/webtron

phony:
	@echo -ne "\033[0;33mAvailable commands:\033[0m\n\n\
make install\t- installs the server executable to $(GOPATH)/bin\n\
make test\t- removes, builds then runs the server executable (debug mode)\n\
make uninstall\t- uninstalls the server executable from $(GOPATH)/bin\n\
\n\
make build\t- builds the server executable\n\
make run\t- runs the server executable\n\
make run-debug\t- runs the server executable (debug mode)\n\
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

run-debug:
ifneq "$(PORT)" ""
	@if [ -f $(WEBTRONPATH)/webtron ]; then \
	echo "Running webtron on port $(PORT)"; \
	$(WEBTRONPATH)/webtron -listenPort $(PORT) -debug; fi
else
	@if [ -f $(WEBTRONPATH)/webtron ]; then \
	echo "Running webtron on default port"; \
	$(WEBTRONPATH)/webtron -debug; fi
endif

clean:
	@if [ -f $(WEBTRONPATH)/webtron ]; then \
	echo "Removing webtron"; \
	rm -f $(WEBTRONPATH)/webtron; fi

install:
	@echo "Installing webtron"
	@go install go.owls.io/webtron
	@cp -r ./client $(GOPATH)/bin/

uninstall:
	@echo "Uninstalling webtron"
	@rm $(GOPATH)/bin/webtron
	@rm -r $(GOPATH)/bin/client

test: clean build run-debug
