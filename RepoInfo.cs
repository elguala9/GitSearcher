using System.Text.Json.Serialization;

namespace GitSearcher;

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
