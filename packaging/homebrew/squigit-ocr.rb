class SquigitOcr < Formula
  desc "Standalone purely headless CLI OCR engine for Squigit"
  homepage "https://github.com/a7mddra/squigit"
  # URL and sha256 are to be auto-filled by the CI deployment pipeline
  url "https://github.com/a7mddra/squigit/releases/download/v0.1.0/squigit-ocr-mac-aarch64.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  version "0.1.0"

  def install
    # Bypassing gatekeeper natively through homebrew
    bin.install "squigit-ocr" => "squigit-ocr"
    if Dir.exist?("_internal")
      prefix.install "_internal"
    end
  end

  test do
    system "#{bin}/squigit-ocr", "--help"
  end
end
