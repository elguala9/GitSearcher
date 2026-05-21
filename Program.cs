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

            GitScanner.ScanDirectory(opts.RootPath, opts, repos, ref scanned);

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
}
