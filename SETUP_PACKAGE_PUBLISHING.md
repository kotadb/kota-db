# Package Publishing Setup Instructions

## ⚠️ IMPORTANT: Add GitHub Secrets

You need to add these secrets to your GitHub repository for automated package publishing to work.

### Step 1: Navigate to Repository Settings
1. Go to https://github.com/jayminwest/kota-db
2. Click on "Settings" tab
3. In the left sidebar, click on "Secrets and variables" → "Actions"
4. Click "New repository secret"

### Step 2: Add PyPI Token
**Secret Name:** `PYPI_API_TOKEN`
**Secret Value:** Your PyPI API token (starts with `pypi-`)

⚠️ **IMPORTANT**: These secrets have already been configured for this repository.

### Step 3: Add npm Token
**Secret Name:** `NPM_TOKEN`
**Secret Value:** Your npm access token (starts with `npm_`)

## Testing the Workflow

Once you've added both secrets, you can test the publishing workflow:

### Option 1: Manual Test (Recommended for First Time)
1. Go to Actions tab in your repository
2. Select "Publish Client Libraries" workflow
3. Click "Run workflow"
4. Enter version "0.2.0" 
5. Check both publishing options
6. Click "Run workflow"

### Option 2: Create a Test Release
```bash
# Create a test tag
git tag v0.2.1-test
git push origin v0.2.1-test

# Create a pre-release on GitHub
gh release create v0.2.1-test --prerelease --title "Test Release" --notes "Testing package publishing"
```

## Verify Published Packages

After successful publishing:

### Python Package
- Visit: https://pypi.org/project/kotadb-client/
- Test installation: `pip install kotadb-client`

### TypeScript Package  
- Visit: https://www.npmjs.com/package/kotadb-client
- Test installation: `npm install kotadb-client`

## Troubleshooting

### If PyPI publishing fails:
1. Check that the package name `kotadb-client` is available
2. Verify the token has not expired
3. Ensure the token is correctly added as a secret

### If npm publishing fails:
1. Check that the package name `kotadb-client` is available
2. Verify the token has publish permissions
3. Ensure you're logged into npm with the correct account

## Next Steps

After successful test:
1. Delete test releases if created
2. The workflow will automatically run for all future releases
3. Consider adding package badges to README.md:

```markdown
[![PyPI version](https://badge.fury.io/py/kotadb-client.svg)](https://pypi.org/project/kotadb-client/)
[![npm version](https://badge.fury.io/js/kotadb-client.svg)](https://www.npmjs.com/package/kotadb-client)
```

## Security Notes

⚠️ **NEVER** commit these tokens to the repository
⚠️ **NEVER** share these tokens publicly
⚠️ Consider rotating tokens periodically for security

These tokens are scoped to only allow package publishing and cannot modify your account settings.