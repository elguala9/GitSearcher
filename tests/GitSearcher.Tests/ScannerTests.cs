using System.Runtime.InteropServices;
using Xunit;

namespace GitSearcher.Tests;

public class ScannerTests
{
    // Absolute path to the fixtures folder committed alongside the tests.
    private static readonly string FixturesRoot = Path.GetFullPath(
        Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "fixtures"));

    private static CliOptions DefaultOpts(string root, string output) => new()
    {
        RootPath  = root,
        OutputPath = output,
        IncludeHidden = false,
    };

    private static CliOptions HiddenOpts(string root, string output) => new()
    {
        RootPath  = root,
        OutputPath = output,
        IncludeHidden = true,
    };

    [Fact]
    public void Finds_RepoAlpha_WithMainBranch()
    {
        var repos = Scan(DefaultOpts(FixturesRoot, TmpJson()));
        var alpha = repos.FirstOrDefault(r => r.Path.EndsWith("repo-alpha", Comparison));
        Assert.NotNull(alpha);
        Assert.Equal("main", alpha.Branch);
    }

    [Fact]
    public void Finds_RepoBeta_AsDetached()
    {
        var repos = Scan(DefaultOpts(FixturesRoot, TmpJson()));
        var beta = repos.FirstOrDefault(r => r.Path.EndsWith("repo-beta", Comparison));
        Assert.NotNull(beta);
        Assert.StartsWith("detached@", beta.Branch);
    }

    [Fact]
    public void Finds_NestedRepoGamma_WithDevelopBranch()
    {
        var repos = Scan(DefaultOpts(FixturesRoot, TmpJson()));
        var gamma = repos.FirstOrDefault(r => r.Path.EndsWith("repo-gamma", Comparison));
        Assert.NotNull(gamma);
        Assert.Equal("develop", gamma.Branch);
    }

    [Fact]
    public void Skips_HiddenRepo_ByDefault()
    {
        var repos = Scan(DefaultOpts(FixturesRoot, TmpJson()));
        Assert.DoesNotContain(repos, r => r.Path.Contains(".hidden-repo", Comparison));
    }

    [Fact]
    public void Skips_NodeModulesRepo_ByDefault()
    {
        var repos = Scan(DefaultOpts(FixturesRoot, TmpJson()));
        Assert.DoesNotContain(repos, r => r.Path.Contains("node_modules", Comparison));
    }

    [Fact]
    public void Includes_HiddenRepo_WithFlag()
    {
        var repos = Scan(HiddenOpts(FixturesRoot, TmpJson()));
        Assert.Contains(repos, r => r.Path.Contains(".hidden-repo", Comparison));
    }

    [Fact]
    public void Includes_NodeModulesRepo_WithFlag()
    {
        var repos = Scan(HiddenOpts(FixturesRoot, TmpJson()));
        Assert.Contains(repos, r => r.Path.Contains("node_modules", Comparison));
    }

    [Fact]
    public void Source_IsLogsHead_WhenFileExists()
    {
        var repos = Scan(DefaultOpts(FixturesRoot, TmpJson()));
        var alpha = repos.First(r => r.Path.EndsWith("repo-alpha", Comparison));
        Assert.Equal("logs/HEAD", alpha.Source);
    }

    [Fact]
    public void LastActivityUtc_IsSet()
    {
        var repos = Scan(DefaultOpts(FixturesRoot, TmpJson()));
        Assert.All(repos, r => Assert.NotNull(r.LastActivityUtc));
    }

    // ── helpers ────────────────────────────────────────────────────────────

    private static StringComparison Comparison =>
        RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? StringComparison.OrdinalIgnoreCase
            : StringComparison.Ordinal;

    private static List<RepoInfo> Scan(CliOptions opts)
    {
        var repos = new List<RepoInfo>();
        int scanned = 0;
        GitScanner.ScanDirectory(opts.RootPath, opts, repos, ref scanned);
        return repos;
    }

    private static string TmpJson() =>
        Path.Combine(Path.GetTempPath(), $"gitsearcher-test-{Guid.NewGuid():N}.json");
}
