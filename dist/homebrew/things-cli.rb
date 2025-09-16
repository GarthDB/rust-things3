class ThingsCli < Formula
  desc "Command-line interface for Things 3 with integrated MCP server"
  homepage "https://github.com/GarthDB/rust-things"
  url "https://github.com/GarthDB/rust-things/archive/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"
  head "https://github.com/GarthDB/rust-things.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release", "--bin", "things-cli"
    bin.install "target/release/things-cli" => "things-cli"
  end

  test do
    # Test basic functionality
    assert_match "Things CLI", shell_output("#{bin}/things-cli --help")
    
    # Test MCP server startup (should show available tools)
    output = shell_output("#{bin}/things-cli mcp --help", 1)
    assert_match "MCP server", output
  end
end
