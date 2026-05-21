namespace GitSearcher;

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
