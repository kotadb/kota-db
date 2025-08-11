.PHONY: help install install-dev clean lint format test test-unit test-integration test-stress test-coverage check security benchmark docs build publish

PYTHON := python3
PIP := $(PYTHON) -m pip
PYTEST := $(PYTHON) -m pytest
COVERAGE := $(PYTHON) -m coverage

# Colors for output
RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[0;33m
BLUE := \033[0;34m
NC := \033[0m # No Color

help: ## Show this help message
	@echo "$(BLUE)KotaDB Python Client Development Commands$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "$(GREEN)%-20s$(NC) %s\n", $$1, $$2}'

install: ## Install package in production mode
	$(PIP) install -e .
	@echo "$(GREEN)✓ Package installed$(NC)"

install-dev: ## Install package with all development dependencies
	$(PIP) install -e ".[dev,test]"
	@echo "$(GREEN)✓ Development dependencies installed$(NC)"

clean: ## Clean build artifacts and cache files
	rm -rf build/ dist/ *.egg-info
	rm -rf .pytest_cache .coverage htmlcov/ .mypy_cache .ruff_cache
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete
	find . -type f -name "*.pyo" -delete
	@echo "$(GREEN)✓ Cleaned build artifacts$(NC)"

# Formatting and Linting
format: ## Format code with black and isort
	$(PYTHON) -m black kotadb tests
	$(PYTHON) -m isort kotadb tests
	@echo "$(GREEN)✓ Code formatted$(NC)"

lint: ## Run all linters (ruff, mypy, pylint)
	@echo "$(BLUE)Running ruff...$(NC)"
	$(PYTHON) -m ruff check kotadb tests
	@echo "$(BLUE)Running mypy...$(NC)"
	$(PYTHON) -m mypy kotadb
	@echo "$(BLUE)Running pylint...$(NC)"
	$(PYTHON) -m pylint kotadb
	@echo "$(GREEN)✓ All linting checks passed$(NC)"

lint-fix: ## Fix auto-fixable linting issues
	$(PYTHON) -m ruff check --fix kotadb tests
	$(PYTHON) -m black kotadb tests
	$(PYTHON) -m isort kotadb tests
	@echo "$(GREEN)✓ Fixed linting issues$(NC)"

# Testing
test: ## Run all tests with coverage
	$(PYTEST) --cov=kotadb --cov-report=term-missing --cov-report=html --cov-fail-under=90
	@echo "$(GREEN)✓ All tests passed$(NC)"

test-unit: ## Run unit tests only
	$(PYTEST) tests/ -m "unit" -v
	@echo "$(GREEN)✓ Unit tests passed$(NC)"

test-integration: ## Run integration tests against real server
	@echo "$(YELLOW)Starting test server...$(NC)"
	@cd ../.. && cargo run --release --bin kotadb -- serve --port 8080 > /tmp/kotadb-test.log 2>&1 & echo $$! > /tmp/kotadb-test.pid
	@sleep 2
	$(PYTEST) tests/ -m "integration" -v || (kill `cat /tmp/kotadb-test.pid` 2>/dev/null; false)
	@kill `cat /tmp/kotadb-test.pid` 2>/dev/null || true
	@rm -f /tmp/kotadb-test.pid
	@echo "$(GREEN)✓ Integration tests passed$(NC)"

test-property: ## Run property-based tests with hypothesis
	$(PYTEST) tests/test_property.py -v --hypothesis-show-statistics
	@echo "$(GREEN)✓ Property tests passed$(NC)"

test-stress: ## Run stress tests
	$(PYTEST) tests/test_stress.py -v -m "slow"
	@echo "$(GREEN)✓ Stress tests passed$(NC)"

test-coverage: ## Generate coverage report
	$(COVERAGE) run -m pytest tests/
	$(COVERAGE) report
	$(COVERAGE) html
	@echo "$(GREEN)✓ Coverage report generated in htmlcov/$(NC)"

test-watch: ## Run tests in watch mode
	$(PYTHON) -m pytest_watch -- -v

# Security
security: ## Run security checks
	@echo "$(BLUE)Running bandit security scan...$(NC)"
	$(PYTHON) -m bandit -r kotadb -f json -o security-report.json
	@echo "$(BLUE)Checking dependency vulnerabilities...$(NC)"
	$(PYTHON) -m safety check --json
	@echo "$(GREEN)✓ Security checks passed$(NC)"

# Performance
benchmark: ## Run performance benchmarks
	$(PYTEST) tests/test_benchmark.py -v --benchmark-only --benchmark-autosave
	@echo "$(GREEN)✓ Benchmarks completed$(NC)"

# Quality Gates (mimics Rust's 'just check')
check: format lint test security ## Run all quality checks (format, lint, test, security)
	@echo "$(GREEN)✅ All quality checks passed!$(NC)"

check-strict: ## Run strict checks (no auto-fixing)
	$(PYTHON) -m black --check kotadb tests
	$(PYTHON) -m isort --check-only kotadb tests
	$(PYTHON) -m ruff check kotadb tests
	$(PYTHON) -m mypy kotadb --strict
	$(PYTHON) -m pylint kotadb --fail-under=9.0
	$(PYTEST) --cov=kotadb --cov-fail-under=95
	$(PYTHON) -m bandit -r kotadb -ll
	@echo "$(GREEN)✅ All strict checks passed!$(NC)"

# Documentation
docs: ## Build documentation
	cd docs && $(PYTHON) -m sphinx-build -b html . _build/html
	@echo "$(GREEN)✓ Documentation built in docs/_build/html$(NC)"

# Build and Publish
build: clean ## Build distribution packages
	$(PYTHON) -m build
	@echo "$(GREEN)✓ Distribution packages built$(NC)"

publish-test: build ## Publish to TestPyPI
	$(PYTHON) -m twine upload --repository testpypi dist/*
	@echo "$(GREEN)✓ Published to TestPyPI$(NC)"

publish: build ## Publish to PyPI
	$(PYTHON) -m twine upload dist/*
	@echo "$(GREEN)✓ Published to PyPI$(NC)"

# Development helpers
shell: ## Start Python shell with kotadb imported
	$(PYTHON) -c "from kotadb import KotaDB; import IPython; IPython.embed()"

dev-server: ## Start KotaDB server for development
	cd ../.. && cargo run --release --bin kotadb -- serve --port 8080

# CI/CD helpers
ci: check-strict ## Run CI pipeline locally
	@echo "$(GREEN)✅ CI pipeline passed!$(NC)"

pre-commit: format lint test ## Run pre-commit checks
	@echo "$(GREEN)✅ Pre-commit checks passed!$(NC)"