VERSION := 0.1.0

srpm:
	dnf install -y rpm-build rpmdevtools git
	mkdir -p /tmp/memguard-rpm/{SOURCES,SRPMS}
	git archive --prefix=memguard-$(VERSION)/ HEAD | gzip > /tmp/memguard-rpm/SOURCES/memguard-$(VERSION).tar.gz
	rpmbuild -bs --define "_topdir /tmp/memguard-rpm" memguard.spec
	cp /tmp/memguard-rpm/SRPMS/*.src.rpm $(outdir)
