class SquigitStt < Formula
  desc "Standalone purely headless CLI STT engine for Squigit"
  homepage "https://github.com/squigit-org/squigit"
  # Source template: release CI copies this file into the tap repo, then fills metadata via pkg.rb.
  url "INSERT_URL_HERE"
  sha256 "INSERT_SHA256_HERE"
  version "INSERT_VERSION_HERE"

  def install
    unless OS.mac? && Hardware::CPU.arm?
      odie "squigit-stt currently supports macOS arm64 (Apple Silicon) only."
    end

    bin.install "squigit-stt" => "squigit-stt"
    (share/"squigit-stt/models").install Dir["models/*"] if Dir.exist?("models")
    prefix.install "_internal" if Dir.exist?("_internal")
  end

  test do
    system "#{bin}/squigit-stt", "--help"
    system "#{bin}/squigit-stt", "--version"
  end
end
