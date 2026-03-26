Name:           squigit-ocr
Version:        0.1.0
Release:        1%{?dist}
Summary:        Standalone CLI OCR engine for Squigit

License:        Apache-2.0
URL:            https://github.com/a7mddra/squigit

%description
A headless, offline optical character recognition pipeline designed for Squigit.

%install
mkdir -p %{buildroot}/usr/bin
cp -a squigit-ocr/* %{buildroot}/usr/bin/

%files
/usr/bin/*
