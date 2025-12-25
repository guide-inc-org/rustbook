// GitBook-compatible JavaScript

(function() {
    'use strict';

    // TOC toggle functionality
    var tocToggle = document.querySelector('.toc-toggle');
    var book = document.querySelector('.book');
    var pageToc = document.querySelector('.page-toc');

    function isMobileToc() {
        return window.innerWidth <= 768;
    }

    if (tocToggle && book && pageToc) {
        // Restore TOC state from localStorage (desktop only)
        if (!isMobileToc()) {
            var tocHidden = localStorage.getItem('guidebook-toc-hidden') === 'true';
            if (tocHidden) {
                book.classList.add('toc-hidden');
            }
        }

        tocToggle.addEventListener('click', function() {
            if (!isMobileToc()) {
                book.classList.toggle('toc-hidden');
                var isHidden = book.classList.contains('toc-hidden');
                localStorage.setItem('guidebook-toc-hidden', isHidden);
            }
        });

        // Handle resize
        window.addEventListener('resize', function() {
            if (isMobileToc()) {
                // On mobile, TOC is always hidden via CSS
            } else {
                // On desktop, restore saved state
                var tocHidden = localStorage.getItem('guidebook-toc-hidden') === 'true';
                if (tocHidden) {
                    book.classList.add('toc-hidden');
                } else {
                    book.classList.remove('toc-hidden');
                }
            }
        });
    }

    // TOC scroll spy - highlight current section
    function setupTocScrollSpy() {
        var tocLinks = document.querySelectorAll('.page-toc .toc-list a');
        if (tocLinks.length === 0) return;

        var headings = [];
        tocLinks.forEach(function(link) {
            var href = link.getAttribute('href');
            if (href && href.startsWith('#')) {
                var id = href.substring(1);
                try {
                    id = decodeURIComponent(id);
                } catch (e) {}
                var heading = document.getElementById(id);
                if (heading) {
                    headings.push({ element: heading, link: link });
                }
            }
        });

        if (headings.length === 0) return;

        function updateActiveLink() {
            var scrollTop = window.scrollY + 100; // Offset for fixed header
            var activeIndex = 0;

            for (var i = 0; i < headings.length; i++) {
                if (headings[i].element.offsetTop <= scrollTop) {
                    activeIndex = i;
                }
            }

            tocLinks.forEach(function(link) {
                link.parentElement.classList.remove('active');
            });
            headings[activeIndex].link.parentElement.classList.add('active');
        }

        window.addEventListener('scroll', updateActiveLink);
        updateActiveLink(); // Initial call
    }

    setupTocScrollSpy();

    // TOC link click handler - prevent base href issue
    function setupTocLinks() {
        var pageToc = document.querySelector('.page-toc');
        if (!pageToc) return;

        pageToc.addEventListener('click', function(e) {
            var link = e.target.closest('a');
            if (!link) return;

            var href = link.getAttribute('href');
            if (!href || !href.startsWith('#')) return;

            e.preventDefault();

            var id = href.substring(1);
            try {
                id = decodeURIComponent(id);
            } catch (ex) {}

            var target = document.getElementById(id);
            if (target) {
                target.scrollIntoView({ behavior: 'smooth' });
                history.pushState(null, '', href);
            }
        });
    }

    setupTocLinks();

    // Back to top button
    var backToTop = document.querySelector('.back-to-top');
    if (backToTop) {
        window.addEventListener('scroll', function() {
            if (window.scrollY > 300) {
                backToTop.classList.add('visible');
            } else {
                backToTop.classList.remove('visible');
            }
        });

        backToTop.addEventListener('click', function(e) {
            e.preventDefault();
            window.scrollTo({ top: 0, behavior: 'smooth' });
        });
    }

    // Sidebar toggle
    var sidebarToggle = document.querySelector('.sidebar-toggle');
    var book = document.querySelector('.book');
    var bookSummary = document.querySelector('.book-summary');

    function isMobile() {
        return window.innerWidth <= 768;
    }

    var wasMobile = isMobile();

    if (sidebarToggle && book && bookSummary) {
        // Restore sidebar state from localStorage (desktop only)
        if (!isMobile()) {
            var sidebarHidden = localStorage.getItem('guidebook-sidebar-hidden') === 'true';
            if (sidebarHidden) {
                book.classList.add('sidebar-hidden');
            }
        }

        sidebarToggle.addEventListener('click', function() {
            if (isMobile()) {
                // Mobile: toggle .open on sidebar
                bookSummary.classList.toggle('open');
            } else {
                // Desktop: toggle .sidebar-hidden on book
                book.classList.add('sidebar-toggling');
                book.classList.toggle('sidebar-hidden');
                var isHidden = book.classList.contains('sidebar-hidden');
                localStorage.setItem('guidebook-sidebar-hidden', isHidden);
                setTimeout(function() {
                    book.classList.remove('sidebar-toggling');
                }, 350);
            }
        });

        // Close sidebar when clicking outside on mobile
        document.addEventListener('click', function(e) {
            if (isMobile() && bookSummary.classList.contains('open')) {
                if (!bookSummary.contains(e.target) && !sidebarToggle.contains(e.target)) {
                    bookSummary.classList.remove('open');
                }
            }
        });

        // Handle resize: switch between mobile and desktop modes
        window.addEventListener('resize', function() {
            var nowMobile = isMobile();
            if (wasMobile !== nowMobile) {
                if (nowMobile) {
                    // Switched to mobile: reset desktop state, close sidebar
                    book.classList.remove('sidebar-hidden');
                    book.classList.remove('sidebar-toggling');
                    bookSummary.classList.remove('open');
                } else {
                    // Switched to desktop: reset mobile state, restore desktop state
                    bookSummary.classList.remove('open');
                    var sidebarHidden = localStorage.getItem('guidebook-sidebar-hidden') === 'true';
                    if (sidebarHidden) {
                        book.classList.add('sidebar-hidden');
                    } else {
                        book.classList.remove('sidebar-hidden');
                    }
                }
                wasMobile = nowMobile;
            }
        });
    }

    // Smooth scroll for anchor links
    document.querySelectorAll('a[href*="#"]').forEach(function(anchor) {
        anchor.addEventListener('click', function(e) {
            var href = this.getAttribute('href');
            var hashIndex = href.indexOf('#');
            if (hashIndex === -1) return;

            var hash = href.substring(hashIndex + 1);
            // Decode URL-encoded anchor (e.g., %E3%83%87%E3%82%B6%E3%82%A4%E3%83%B3 -> デザイン)
            try {
                hash = decodeURIComponent(hash);
            } catch (ex) {
                // If decoding fails, use as-is
            }

            var target = document.getElementById(hash);
            if (target) {
                e.preventDefault();
                target.scrollIntoView({ behavior: 'smooth' });
                // Update URL hash without triggering navigation
                history.pushState(null, '', '#' + encodeURIComponent(hash));
            }
        });
    });

    // SPA-like navigation for sidebar links
    // Store base URL on initial page load (e.g., /jp/)
    var baseUrl = (function() {
        var base = document.querySelector('base');
        if (base && base.href) {
            return base.href;
        }
        return window.location.href.replace(/[^/]*$/, '');
    })();

    // Prevent rapid navigation
    var isNavigating = false;

    function setupSpaNavigation() {
        var sidebar = document.querySelector('.book-summary');
        if (!sidebar) return;

        sidebar.addEventListener('click', function(e) {
            var link = e.target.closest('a');
            if (!link) return;

            var href = link.getAttribute('href');
            if (!href || href.startsWith('#') || href.startsWith('http')) return;

            e.preventDefault();
            if (isNavigating) return;
            loadPage(href, link);
        });
    }

    // Setup page navigation (prev/next buttons)
    function setupPageNavigation() {
        document.querySelectorAll('.page-nav').forEach(function(nav) {
            nav.addEventListener('click', function(e) {
                e.preventDefault();
                if (isNavigating) return;

                var href = this.getAttribute('href');
                if (!href) return;

                loadPage(href, null);
            });
        });
    }

    function loadPage(url, clickedLink) {
        if (isNavigating) return;
        isNavigating = true;

        // Add loading state
        document.body.classList.add('loading');

        // Always resolve relative to the fixed base URL
        var absoluteUrl = new URL(url, baseUrl).href;

        // Extract hash from URL if present
        var hashIndex = url.indexOf('#');
        var hash = hashIndex !== -1 ? url.substring(hashIndex + 1) : null;

        fetch(absoluteUrl)
            .then(function(response) {
                if (!response.ok) throw new Error('Page not found');
                return response.text();
            })
            .then(function(html) {
                var parser = new DOMParser();
                var doc = parser.parseFromString(html, 'text/html');

                // Update content
                var newContent = doc.querySelector('.markdown-section');
                var currentContent = document.querySelector('.markdown-section');
                if (newContent && currentContent) {
                    currentContent.innerHTML = newContent.innerHTML;
                }

                // Update title
                var newTitle = doc.querySelector('title');
                if (newTitle) {
                    document.title = newTitle.textContent;
                }

                // Update active state in sidebar (don't replace HTML to preserve expanded state)
                document.querySelectorAll('.book-summary .chapter.active').forEach(function(ch) {
                    ch.classList.remove('active');
                });

                // Find and mark new active item
                var newActiveHref = url.replace(/^\.\//, '').split('#')[0];
                document.querySelectorAll('.book-summary .chapter a').forEach(function(link) {
                    var href = link.getAttribute('href');
                    if (href === newActiveHref || href === './' + newActiveHref) {
                        var chapter = link.closest('.chapter');
                        if (chapter) {
                            chapter.classList.add('active');
                            // Expand parent chapters
                            var parent = chapter.parentElement;
                            while (parent) {
                                if (parent.classList && parent.classList.contains('chapter')) {
                                    parent.classList.add('expanded');
                                }
                                parent = parent.parentElement;
                            }
                        }
                    }
                });

                // Scroll active item into view
                setTimeout(function() {
                    var activeItem = document.querySelector('.book-summary .chapter.active');
                    if (activeItem) {
                        activeItem.scrollIntoView({ block: 'center', behavior: 'auto' });
                    }
                }, 50);

                // Update URL (use absolute URL to avoid relative path issues with SPA navigation)
                history.pushState(null, '', absoluteUrl);

                // Scroll to hash anchor or top
                if (hash) {
                    try {
                        var decodedHash = decodeURIComponent(hash);
                        var target = document.getElementById(decodedHash);
                        if (target) {
                            setTimeout(function() {
                                target.scrollIntoView({ behavior: 'auto' });
                            }, 50);
                        } else {
                            window.scrollTo(0, 0);
                        }
                    } catch (ex) {
                        window.scrollTo(0, 0);
                    }
                } else {
                    window.scrollTo(0, 0);
                }

                // Re-init mermaid if present
                if (typeof mermaid !== 'undefined') {
                    mermaid.init(undefined, '.markdown-section .mermaid');
                }

                // Update TOC from new page
                var newToc = doc.querySelector('.page-toc');
                var currentToc = document.querySelector('.page-toc');
                var newTocToggle = doc.querySelector('.toc-toggle');
                var currentTocToggle = document.querySelector('.toc-toggle');

                if (currentToc) currentToc.remove();
                if (currentTocToggle) currentTocToggle.remove();

                if (newToc) {
                    var bookBody = document.querySelector('.book-body');
                    if (bookBody) {
                        var tocClone = newToc.cloneNode(true);
                        bookBody.insertBefore(tocClone, bookBody.querySelector('.body-inner'));
                        if (newTocToggle) {
                            var toggleClone = newTocToggle.cloneNode(true);
                            bookBody.insertBefore(toggleClone, tocClone);
                            // Re-setup toggle handler
                            toggleClone.addEventListener('click', function() {
                                if (window.innerWidth > 768) {
                                    document.querySelector('.book').classList.toggle('toc-hidden');
                                    var isHidden = document.querySelector('.book').classList.contains('toc-hidden');
                                    localStorage.setItem('guidebook-toc-hidden', isHidden);
                                }
                            });
                        }
                        // Re-setup scroll spy and TOC links
                        setupTocScrollSpy();
                        setupTocLinks();
                    }
                }

                // Update prev/next navigation buttons
                var newPrev = doc.querySelector('.page-nav.prev');
                var newNext = doc.querySelector('.page-nav.next');
                var currentPrev = document.querySelector('.page-nav.prev');
                var currentNext = document.querySelector('.page-nav.next');

                if (currentPrev) currentPrev.remove();
                if (currentNext) currentNext.remove();

                var bodyInner = document.querySelector('.body-inner');
                if (bodyInner) {
                    if (newPrev) {
                        var prevClone = newPrev.cloneNode(true);
                        bodyInner.insertBefore(prevClone, bodyInner.firstChild);
                    }
                    if (newNext) {
                        var nextClone = newNext.cloneNode(true);
                        bodyInner.insertBefore(nextClone, bodyInner.querySelector('.page-wrapper'));
                    }
                    // Re-setup page navigation for new buttons
                    setupPageNavigation();
                }

                // Reset navigation state
                isNavigating = false;
                document.body.classList.remove('loading');
            })
            .catch(function(err) {
                console.error('Navigation error:', err);
                isNavigating = false;
                document.body.classList.remove('loading');
                window.location.href = url;
            });
    }

    // Handle browser back/forward
    window.addEventListener('popstate', function() {
        loadPage(location.pathname + location.hash, null);
    });

    setupSpaNavigation();
    setupPageNavigation();

    // Handle initial page load with hash anchor
    function scrollToHashOnLoad() {
        if (!window.location.hash) return;

        var hash = window.location.hash.substring(1);
        // Decode URL-encoded anchor
        try {
            hash = decodeURIComponent(hash);
        } catch (ex) {
            // If decoding fails, use as-is
        }

        var target = document.getElementById(hash);
        if (target) {
            // Use setTimeout to ensure layout is complete after all resources load
            setTimeout(function() {
                target.scrollIntoView({ behavior: 'auto' });
            }, 100);
        }
    }

    // Use 'load' event to ensure all resources (images, CSS) are loaded
    if (document.readyState === 'complete') {
        scrollToHashOnLoad();
    } else {
        window.addEventListener('load', scrollToHashOnLoad);
    }

    // Scroll active item into view in sidebar
    function scrollActiveIntoView() {
        var activeItem = document.querySelector('.book-summary .chapter.active');
        if (activeItem) {
            var sidebar = document.querySelector('.book-summary');
            if (sidebar) {
                // Get position relative to sidebar
                var itemRect = activeItem.getBoundingClientRect();
                var sidebarRect = sidebar.getBoundingClientRect();

                // Check if item is outside visible area
                if (itemRect.top < sidebarRect.top || itemRect.bottom > sidebarRect.bottom) {
                    activeItem.scrollIntoView({ block: 'center', behavior: 'auto' });
                }
            }
        }
    }

    // Scroll to active item on page load
    setTimeout(scrollActiveIntoView, 50);
})();
