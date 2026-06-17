VERSION := 0.1.0
RELEASE := 1

RPMBUILD := $(HOME)/rpmbuild

.PHONY: all rpm srpm clean

all:
	cargo build --release -p memguard

srpm:
	mkdir -p $(RPMBUILD)/{SOURCES,SRPMS}
	git archive --prefix=memguard-$(VERSION)/ HEAD | gzip > $(RPMBUILD)/SOURCES/memguard-$(VERSION).tar.gz
	rpmbuild -bs --define "_topdir $(RPMBUILD)" memguard.spec

rpm: srpm
	rpmbuild --rebuild $(RPMBUILD)/SRPMS/memguard-$(VERSION)-$(RELEASE).*.src.rpm

clean:
	rm -rf $(RPMBUILD)
	cargo clean
