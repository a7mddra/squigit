Name:           squigit-stt
Version:        0.1.0
Release:        1%{?dist}
Summary:        Standalone CLI STT engine for Squigit

License:        Apache-2.0
URL:            https://github.com/a7mddra/squigit

%description
A headless, offline speech to text whisper-based engine designed for Squigit.

%install
mkdir -p %{buildroot}/usr/bin
cp -a squigit-stt/* %{buildroot}/usr/bin/

%files
/usr/bin/*
