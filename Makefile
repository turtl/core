.PHONY: all clean release build test doc

# non-versioned include
VARS ?= vars.mk
-include $(VARS)

CARGO ?= $(shell which cargo)
FEATURES ?= 
override CARGO_BUILD_ARGS += --features "$(FEATURES)"

all: build

build: 
	$(CARGO) build $(CARGO_BUILD_ARGS)

release: override CARGO_BUILD_ARGS += --release
release: build

test:
	$(CARGO) test $(TEST) $(CARGO_BUILD_ARGS) -- --nocapture

doc:
	$(CARGO) doc -p turtl-core --no-deps

clean:
	rm -rf target/
	cargo clean

