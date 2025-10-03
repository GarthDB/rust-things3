class Things3Cli < Formula
  desc "CLI tool for Things 3 with integrated MCP server"
  homepage "https://github.com/GarthDB/rust-things3"
  url "https://github.com/GarthDB/rust-things3/archive/v0.2.0.tar.gz"
  sha256 "" # This will need to be filled in after the release
  license "MIT"
  head "https://github.com/GarthDB/rust-things3.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "apps/things3-cli")
  end

  test do
    system "#{bin}/things3", "--version"
  end
end
