Name:           __PACKAGE_NAME__
Version:        __VERSION__
Release:        1%{?dist}
Summary:        __SUMMARY__

License:        Apache-2.0
URL:            https://github.com/squigit-org/squigit
BuildArch:      x86_64
Source0:        %{name}-%{version}.tar.gz
AutoReqProv:    no

%description
__DESCRIPTION__

%prep
%setup -q

%build

%install
mkdir -p %{buildroot}
cp -a usr %{buildroot}/

%files
/usr/bin/__BINARY_NAME__
/usr/lib/__PACKAGE_NAME__
