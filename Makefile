CARGO := cargo

.PHONY: check fmt fmt-check lint test doc audit

check: fmt-check lint test doc

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

lint:
	$(CARGO) clippy --workspace --all-targets --all-features --locked \
		-- -D warnings -D clippy::pedantic

test:
	$(CARGO) test --workspace --locked

doc:
	RUSTDOCFLAGS="-D warnings" \
		$(CARGO) doc --workspace --all-features --no-deps --locked

audit:
	$(CARGO) audit
