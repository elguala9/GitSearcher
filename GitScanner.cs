namespace GitSearcher;

internal static class GitScanner
{
    public static void ScanDirectory(string root, CliOptions opts, List<RepoInfo> repos, ref int scanned)
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
