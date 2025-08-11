#!/usr/bin/env python3
"""
Connection examples for KotaDB Python client.

This example demonstrates different ways to connect to KotaDB,
including environment variables and connection strings.
"""

import os

from kotadb import KotaDB, connect
from kotadb.exceptions import ConnectionError


def example_direct_url():
    """Connect using direct HTTP URL."""
    print("🔗 Example 1: Direct HTTP URL")
    try:
        db = KotaDB("http://localhost:8080")
        health = db.health()
        print(f"  ✅ Connected successfully: {health.get('status')}")
        db.close()
    except ConnectionError as e:
        print(f"  ❌ Connection failed: {e}")


def example_environment_variable():
    """Connect using environment variable."""
    print("\n🔗 Example 2: Environment Variable")

    # Set environment variable
    os.environ["KOTADB_URL"] = "http://localhost:8080"

    try:
        db = KotaDB()  # Uses KOTADB_URL automatically
        health = db.health()
        print(f"  ✅ Connected using KOTADB_URL: {health.get('status')}")
        db.close()
    except ConnectionError as e:
        print(f"  ❌ Connection failed: {e}")


def example_connection_string():
    """Connect using kotadb:// connection string."""
    print("\n🔗 Example 3: Connection String")
    try:
        db = KotaDB("kotadb://localhost:8080/myapp")
        health = db.health()
        print(f"  ✅ Connected using connection string: {health.get('status')}")
        db.close()
    except ConnectionError as e:
        print(f"  ❌ Connection failed: {e}")


def example_context_manager():
    """Connect using context manager."""
    print("\n🔗 Example 4: Context Manager")
    try:
        with KotaDB("http://localhost:8080") as db:
            health = db.health()
            print(f"  ✅ Connected with context manager: {health.get('status')}")
            # Connection automatically closed when exiting with block
    except ConnectionError as e:
        print(f"  ❌ Connection failed: {e}")


def example_convenience_function():
    """Connect using convenience function."""
    print("\n🔗 Example 5: Convenience Function")
    try:
        db = connect("http://localhost:8080")
        health = db.health()
        print(f"  ✅ Connected using connect(): {health.get('status')}")
        db.close()
    except ConnectionError as e:
        print(f"  ❌ Connection failed: {e}")


def example_custom_configuration():
    """Connect with custom timeout and retry settings."""
    print("\n🔗 Example 6: Custom Configuration")
    try:
        db = KotaDB(
            url="http://localhost:8080",
            timeout=60,  # 60 second timeout
            retries=5,  # 5 retry attempts
        )
        health = db.health()
        print(f"  ✅ Connected with custom config: {health.get('status')}")
        db.close()
    except ConnectionError as e:
        print(f"  ❌ Connection failed: {e}")


def example_multiple_connections():
    """Demonstrate multiple simultaneous connections."""
    print("\n🔗 Example 7: Multiple Connections")

    connections = []
    try:
        # Create multiple connections
        for i in range(3):
            db = KotaDB("http://localhost:8080")
            health = db.health()
            print(f"  ✅ Connection {i+1}: {health.get('status')}")
            connections.append(db)

        # Use connections
        for i, db in enumerate(connections):
            stats = db.stats()
            print(f"  📊 Connection {i+1} stats: {stats.get('document_count', 'unknown')} docs")

    except ConnectionError as e:
        print(f"  ❌ Connection failed: {e}")
    finally:
        # Clean up all connections
        for db in connections:
            db.close()


def main():
    """Run all connection examples."""
    print("KotaDB Python Client - Connection Examples")
    print("=" * 50)

    example_direct_url()
    example_environment_variable()
    example_connection_string()
    example_context_manager()
    example_convenience_function()
    example_custom_configuration()
    example_multiple_connections()

    print("\n✅ All connection examples completed!")
    print("\nNote: These examples require KotaDB server running on localhost:8080")


if __name__ == "__main__":
    main()
