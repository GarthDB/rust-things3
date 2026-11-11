class Things3Cli < Formula
  desc "CLI tool for Things 3 with integrated MCP server"
  homepage "https://github.com/GarthDB/rust-things3"
  url "https://github.com/GarthDB/rust-things3/archive/v2.0.0.tar.gz"
  sha256 "5d07c44a9a4a171bd320d525d59ed220f6321eb606f707acb51a2e5cdee90377" # This will need to be filled in after the release
  license "MIT"
  head "https://github.com/GarthDB/rust-things3.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "apps/things3-cli")
  end

  test do
    system "#{bin}/tasklink", "--version"
  end
end
