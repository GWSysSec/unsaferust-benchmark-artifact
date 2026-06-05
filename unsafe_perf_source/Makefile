.PHONY: all heap cpu coverage counter clean

BUILD := cargo build --release

all:
	@echo "Usage: make [heap|cpu|coverage|counter|both|all_three|all_four|clean]"
	@echo "  heap - Build with heap tracker"
	@echo "  cpu - Build with cpu cycle counter"
	@echo "  coverage - Build with unsafe coverage"
	@echo "  counter - Build with unsafe counter"
	@echo "  clean - Remove build artifacts"

heap:
	cargo clean
	@echo "Building the heap_tracker library..."
	$(BUILD) --features heap_tracker

cpu:
	cargo clean
	@echo "Building the cpu_cycle_counter library..."
	$(BUILD) --features cpu_cycle_counter

coverage:
	cargo clean
	@echo "Building the unsafe_coverage library..."
	$(BUILD) --features unsafe_coverage

counter:
	cargo clean
	@echo "Building the unsafe_counter library..."
	$(BUILD) --features unsafe_counter

clean:
	cargo clean