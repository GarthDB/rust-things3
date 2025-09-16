class Things3Cli < Formula
  desc "Command-line interface for Things 3 with integrated MCP server"
  homepage "https://github.com/GarthDB/rust-things"
  url "https://github.com/GarthDB/rust-things/archive/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"
  head "https://github.com/GarthDB/rust-things.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--path", "apps/things3-cli", "--bin", "things3"
    bin.install "target/release/things3" => "things3"
  end

  test do
    # Test basic functionality
    assert_match "Things 3 CLI", shell_output("#{bin}/things3 --help")
    
    # Test MCP server startup (should show available tools)
    output = shell_output("#{bin}/things3 mcp --help", 1)
    assert_match "MCP server", output
  end
end
