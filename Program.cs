using System.Text.Json;
using System.Text.Json.Serialization;

namespace GitSearcher;

internal static class Program
{
    private static int Main(string[] args)
    {
        try
        {
            var opts = CliOptions.Parse(args, out var helpShown);
            if (opts is null) return helpShown ? 0 : 1;

            if (!Directory.Exists(opts.RootPath))
            {
                Console.Error.WriteLine($"Error: percorso non trovato: {opts.RootPath}");
                return 2;
            }

            Console.Error.WriteLine($"Scansione di: {opts.RootPath}");
            var sw = System.Diagnostics.Stopwatch.StartNew();

            var repos = new List<RepoInfo>();
            int scanned = 0;

            ScanDirectory(opts.RootPath, opts, repos, ref scanned);

            sw.Stop();

            repos.Sort((a, b) => Nullable.Compare(b.LastActivityUtc, a.LastActivityUtc));

            var jsonOptions = new JsonSerializerOptions
            {
                WriteIndented = true,
                DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
                Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
            };

            var json = JsonSerializer.Serialize(repos, jsonOptions);
            File.WriteAllText(opts.OutputPath, json);

            Console.Error.WriteLine(
                $"Trovati {repos.Count} repository in {scanned} cartelle ({sw.Elapsed.TotalSeconds:F1}s).");
            Console.Error.WriteLine($"Output: {Path.GetFullPath(opts.OutputPath)}");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Errore inatteso: {ex.Message}");
            return 99;
        }
    }

    private static void ScanDirectory(string root, CliOptions opts, List<RepoInfo> repos, ref int scanned)
    {
        var stack = new Stack<string>();
        stack.Push(root);

        while (stack.Count > 0)
        {
            var current = stack.Pop();
            scanned++;

            string[] subDirs;
            try
            {
                subDirs = Directory.GetDirectories(current);
            }
            catch (UnauthorizedAccessException)
            {
                Console.Error.WriteLine($"  [skip] accesso negato: {current}");
                continue;
            }
            catch (PathTooLongException)
            {
                Console.Error.WriteLine($"  [skip] path troppo lungo: {current}");
                continue;
            }
            catch (DirectoryNotFoundException)
            {
                continue;
            }
            catch (IOException ex)
            {
                Console.Error.WriteLine($"  [skip] IO error in {current}: {ex.Message}");
                continue;
            }

            var gitDir = subDirs.FirstOrDefault(d =>
                string.Equals(Path.GetFileName(d), ".git", StringComparison.OrdinalIgnoreCase));

            if (gitDir is not null)
            {
                try
                {
                    var info = BuildRepoInfo(current, gitDir);
                    repos.Add(info);
                    Console.Error.WriteLine($"  [repo] {current}");
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  [warn] errore leggendo repo {current}: {ex.Message}");
                }
            }

            foreach (var dir in subDirs)
            {
                var name = Path.GetFileName(dir);

                if (string.Equals(name, ".git", StringComparison.OrdinalIgnoreCase))
                    continue;

                if (!opts.IncludeHidden)
                {
                    if (name.StartsWith('.')) continue;
                    if (string.Equals(name, "node_modules", StringComparison.OrdinalIgnoreCase)) continue;

                    try
                    {
                        var attrs = new DirectoryInfo(dir).Attributes;
                        if ((attrs & FileAttributes.ReparsePoint) != 0) continue;
                        if ((attrs & FileAttributes.Hidden) != 0) continue;
                    }
                    catch
                    {
                        // ignore and try to descend
                    }
                }

                stack.Push(dir);
            }
        }
    }

    private static RepoInfo BuildRepoInfo(string repoPath, string gitDir)
    {
        DateTime? lastActivity = null;
        string? source = null;

        var logsHead = Path.Combine(gitDir, "logs", "HEAD");
        if (File.Exists(logsHead))
        {
            lastActivity = File.GetLastWriteTimeUtc(logsHead);
            source = "logs/HEAD";
        }
        else
        {
            var refsHeads = Path.Combine(gitDir, "refs", "heads");
            if (Directory.Exists(refsHeads))
            {
                DateTime? newest = null;
                foreach (var f in Directory.EnumerateFiles(refsHeads, "*", SearchOption.AllDirectories))
                {
                    var t = File.GetLastWriteTimeUtc(f);
                    if (newest is null || t > newest) newest = t;
                }
                if (newest is not null)
                {
                    lastActivity = newest;
                    source = "refs/heads";
                }
            }
        }

        if (lastActivity is null)
        {
            lastActivity = Directory.GetLastWriteTimeUtc(gitDir);
            source = ".git";
        }

        string? branch = null;
        var headFile = Path.Combine(gitDir, "HEAD");
        if (File.Exists(headFile))
        {
            try
            {
                var head = File.ReadAllText(headFile).Trim();
                if (head.StartsWith("ref:", StringComparison.Ordinal))
                {
                    var refName = head[4..].Trim();
                    const string prefix = "refs/heads/";
                    branch = refName.StartsWith(prefix, StringComparison.Ordinal)
                        ? refName[prefix.Length..]
                        : refName;
                }
                else
                {
                    branch = $"detached@{head[..Math.Min(7, head.Length)]}";
                }
            }
            catch
            {
                // ignore
            }
        }

        return new RepoInfo
        {
            Path = repoPath,
            Branch = branch,
            LastActivityUtc = lastActivity,
            Source = source,
        };
    }
}

internal sealed class RepoInfo
{
    [JsonPropertyName("path")]
    public string Path { get; set; } = "";

    [JsonPropertyName("branch")]
    public string? Branch { get; set; }

    [JsonPropertyName("lastActivityUtc")]
    public DateTime? LastActivityUtc { get; set; }

    [JsonPropertyName("source")]
    public string? Source { get; set; }
}

internal sealed class CliOptions
{
    public string RootPath { get; init; } = "";
    public string OutputPath { get; init; } = "";
    public bool IncludeHidden { get; init; }

    public static CliOptions? Parse(string[] args, out bool helpShown)
    {
        helpShown = false;
        string? root = null;
        string? output = null;
        bool includeHidden = false;

        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            switch (a)
            {
                case "-h":
                case "--help":
                case "/?":
                    PrintHelp();
                    helpShown = true;
                    return null;

                case "-o":
                case "--output":
                    if (i + 1 >= args.Length)
                    {
                        Console.Error.WriteLine("Errore: --output richiede un valore.");
                        return null;
                    }
                    output = args[++i];
                    break;

                case "--include-hidden":
                    includeHidden = true;
                    break;

                default:
                    if (a.StartsWith('-'))
                    {
                        Console.Error.WriteLine($"Errore: opzione sconosciuta '{a}'.");
                        PrintHelp();
                        return null;
                    }
                    if (root is null)
                    {
                        root = a;
                    }
                    else
                    {
                        Console.Error.WriteLine("Errore: percorso specificato più di una volta.");
                        return null;
                    }
                    break;
            }
        }

        root ??= Directory.GetCurrentDirectory();
        root = Path.GetFullPath(root);

        output ??= Path.Combine(Directory.GetCurrentDirectory(), "git-repos.json");

        return new CliOptions
        {
            RootPath = root,
            OutputPath = output,
            IncludeHidden = includeHidden,
        };
    }

    private static void PrintHelp()
    {
        Console.WriteLine(
            """
            GitSearcher - cerca repository git e produce un report JSON.

            Uso:
              GitSearcher.exe [path] [opzioni]

            Argomenti:
              path                       percorso da scansionare (default: directory corrente)

            Opzioni:
              -o, --output <file>        file JSON di output (default: git-repos.json)
                  --include-hidden       includi cartelle nascoste e node_modules
              -h, --help                 mostra questo aiuto
            """);
    }
}
