"""
Setup script for KotaDB Python client.
"""

from setuptools import find_packages, setup

with open("README.md", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="kotadb-client",
    version="0.5.0",
    author="KotaDB Team",
    author_email="support@kotadb.dev",
    description="Python client for KotaDB - PostgreSQL-level ease of use for document database",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/jayminwest/kota-db",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Database",
        "Topic :: Software Development :: Libraries :: Python Modules",
    ],
    python_requires=">=3.8",
    install_requires=[
        "requests>=2.25.0",
        "urllib3>=1.26.0",
    ],
    extras_require={
        "dev": [
            "pytest>=6.0",
            "pytest-cov>=2.0",
            "black>=21.0",
            "flake8>=3.8",
            "mypy>=0.800",
        ],
    },
    keywords="database document-store search semantic-search kotadb",
    project_urls={
        "Bug Reports": "https://github.com/jayminwest/kota-db/issues",
        "Source": "https://github.com/jayminwest/kota-db",
        "Documentation": "https://github.com/jayminwest/kota-db/docs",
    },
)
