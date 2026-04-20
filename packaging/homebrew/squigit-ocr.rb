class SquigitOcr < Formula
  desc "Standalone purely headless CLI OCR engine for Squigit"
  homepage "https://github.com/squigit-org/squigit"
  # Source template: release CI copies this file into the tap repo, then fills metadata via pkg.rb.
  url "INSERT_URL_HERE"
  sha256 "INSERT_SHA256_HERE"
  version "INSERT_VERSION_HERE"

  def install
    unless OS.mac? && Hardware::CPU.arm?
      odie "squigit-ocr currently supports macOS arm64 (Apple Silicon) only."
    end

    libexec.install Dir["*"]
    # Homebrew rewrites Mach-O dylib IDs after install. The vendored pydantic_core
    # extension ships with limited header padding and fails when rewritten to a long
    # path under libexec/_internal/...; keep runtime imports stable via a symlink,
    # but relocate the real file to a much shorter libexec path.
    Dir[(libexec/"_internal/pydantic_core/_pydantic_core.cpython-*-darwin.so").to_s].sort.each_with_index do |path, idx|
      ext = Pathname(path)
      short_name = "p#{idx}.so"
      relocated = libexec/short_name
      mv ext, relocated
      ln_sf Pathname("../../#{short_name}"), ext
    end
    bin.install_symlink libexec/"squigit-ocr"
  end

  test do
    system "#{bin}/squigit-ocr", "--help"
    system "#{bin}/squigit-ocr", "--version"
  end
end
