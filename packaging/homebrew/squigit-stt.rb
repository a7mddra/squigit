class SquigitStt < Formula
  desc "Standalone purely headless CLI STT engine for Squigit"
  homepage "https://github.com/a7mddra/squigit"
  # URL and sha256 are to be auto-filled by the CI deployment pipeline
  url "https://github.com/a7mddra/squigit/releases/download/v0.1.0/squigit-stt-mac-aarch64.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  version "0.1.0"

  def install
    bin.install "squigit-stt" => "squigit-stt"
    if Dir.exist?("models")
      (share/"squigit-stt/models").install Dir["models/*"]
    end
    if Dir.exist?("_internal")
      prefix.install "_internal"
    end
  end

  test do
    system "echo '{\"command\": \"quit\"}' | #{bin}/squigit-stt"
  end
end
