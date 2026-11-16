[CmdletBinding()]
param(
    [ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')]
    [string]$Repository = 'BungeeDEV/note-warden',
    [switch]$Apply
)
$ErrorActionPreference = 'Stop'

function Invoke-GitHub([string]$Method, [string]$Path, $Body = $null) {
    $arguments = @('api', '--method', $Method, $Path)
    $tempFile = $null
    try {
        if ($null -ne $Body) {
            $tempFile = New-TemporaryFile
            $Body | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $tempFile.FullName -Encoding utf8NoBOM
            $arguments += @('--input', $tempFile.FullName)
        }
        $result = & gh @arguments
        if ($LASTEXITCODE) { throw "GitHub API failed: $Method $Path" }
        if ($result) { return ($result | Out-String | ConvertFrom-Json) }
    } finally {
        if ($tempFile) { Remove-Item -LiteralPath $tempFile.FullName }
    }
}

$base = "repos/$Repository"
$repo = Invoke-GitHub GET $base
if (-not $repo.permissions.admin) { throw 'Repository admin access is required.' }
if ($repo.default_branch -ne 'main') { throw 'This configuration expects main as the default branch.' }
if ($repo.private) { throw 'This configuration is intended for the public Note Warden repository.' }
$rulesets = @(Invoke-GitHub GET "$base/rulesets")
$ruleName = 'main: pull requests and green checks'
$existing = @($rulesets | Where-Object { $_.name -eq $ruleName })
if ($existing.Count -gt 1) { throw 'Multiple managed rulesets found; inspect them before applying.' }
if (-not $Apply) {
    Write-Host "Plan for ${Repository}: squash merges, branch cleanup, required PR and CI/CodeQL checks, no force pushes/deletion, main-only release environment, immutable releases, security alerts and SHA-pinned Actions."
    Write-Host 'Run again with -Apply to update GitHub. Other rulesets are retained.'
    return
}

# Save the previous non-secret settings locally before changing anything.
$gitDir = & git -C (Split-Path -Parent $PSScriptRoot) rev-parse --absolute-git-dir
if ($LASTEXITCODE) { throw 'Cannot find the local Git directory for the settings snapshot.' }
$snapshot = @{
    repository = $repo
    rulesets = @($rulesets | ForEach-Object { Invoke-GitHub GET "$base/rulesets/$($_.id)" })
    actions = Invoke-GitHub GET "$base/actions/permissions"
    workflow = Invoke-GitHub GET "$base/actions/permissions/workflow"
    environments = Invoke-GitHub GET "$base/environments"
    immutable_releases = Invoke-GitHub GET "$base/immutable-releases"
}
$snapshotPath = Join-Path $gitDir ('github-settings-before-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.json')
$snapshot | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $snapshotPath -Encoding utf8NoBOM

[void](Invoke-GitHub PATCH $base @{
    description = 'Local Windows app for update plans, notes, tasks and reminders. Rust, GPUI and signed app updates.'
    allow_squash_merge = $true
    allow_merge_commit = $false
    allow_rebase_merge = $false
    delete_branch_on_merge = $true
    allow_update_branch = $true
    allow_auto_merge = $true
    squash_merge_commit_title = 'PR_TITLE'
    squash_merge_commit_message = 'PR_BODY'
    security_and_analysis = @{
        secret_scanning = @{ status = 'enabled' }
        secret_scanning_push_protection = @{ status = 'enabled' }
    }
})
[void](Invoke-GitHub PUT "$base/topics" @{ names = @('windows', 'rust', 'gpui', 'notes', 'task-management', 'desktop-app') })
[void](Invoke-GitHub PUT "$base/vulnerability-alerts")
[void](Invoke-GitHub PUT "$base/automated-security-fixes")
[void](Invoke-GitHub PUT "$base/private-vulnerability-reporting")
[void](Invoke-GitHub PUT "$base/immutable-releases")
[void](Invoke-GitHub PUT "$base/actions/permissions" @{ enabled = $true; allowed_actions = 'all'; sha_pinning_required = $true })
[void](Invoke-GitHub PUT "$base/actions/permissions/workflow" @{ default_workflow_permissions = 'read'; can_approve_pull_request_reviews = $false })

[void](Invoke-GitHub PUT "$base/environments/release" @{
    deployment_branch_policy = @{ protected_branches = $false; custom_branch_policies = $true }
})
$branchPolicies = Invoke-GitHub GET "$base/environments/release/deployment-branch-policies"
if (-not ($branchPolicies.branch_policies | Where-Object { $_.name -eq 'main' -and $_.type -eq 'branch' })) {
    [void](Invoke-GitHub POST "$base/environments/release/deployment-branch-policies" @{ name = 'main'; type = 'branch' })
}

$checks = @('CI passed', 'CodeQL (rust)', 'CodeQL (actions)', 'CodeQL (javascript-typescript)', 'CodeQL (python)')
$ruleset = @{
    name = $ruleName
    target = 'branch'
    enforcement = 'active'
    bypass_actors = @()
    conditions = @{ ref_name = @{ include = @('refs/heads/main'); exclude = @() } }
    rules = @(
        @{ type = 'deletion' },
        @{ type = 'non_fast_forward' },
        @{ type = 'required_linear_history' },
        @{ type = 'code_scanning'; parameters = @{
            code_scanning_tools = @(@{ tool = 'CodeQL'; alerts_threshold = 'errors'; security_alerts_threshold = 'high_or_higher' })
        } },
        @{ type = 'pull_request'; parameters = @{
            required_approving_review_count = 0
            dismiss_stale_reviews_on_push = $true
            require_code_owner_review = $false
            require_last_push_approval = $false
            required_review_thread_resolution = $true
            allowed_merge_methods = @('squash')
        } },
        @{ type = 'required_status_checks'; parameters = @{
            strict_required_status_checks_policy = $true
            do_not_enforce_on_create = $false
            required_status_checks = @($checks | ForEach-Object { @{ context = $_; integration_id = 15368 } })
        } }
    )
}
if ($existing.Count -eq 1) {
    [void](Invoke-GitHub PUT "$base/rulesets/$($existing[0].id)" $ruleset)
} else {
    [void](Invoke-GitHub POST "$base/rulesets" $ruleset)
}
Write-Host "GitHub settings applied to $Repository. Previous settings: $snapshotPath"
