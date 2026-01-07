// tsrun site JavaScript

(function() {
  'use strict';

  // Copy to clipboard
  function copyCode(button) {
    const codeBlock = button.closest('.code-block');
    const code = codeBlock.querySelector('code').textContent;

    navigator.clipboard.writeText(code).then(() => {
      const originalText = button.textContent;
      button.textContent = 'Copied!';
      button.style.color = 'var(--success)';
      setTimeout(() => {
        button.textContent = originalText;
        button.style.color = '';
      }, 2000);
    }).catch(() => {
      button.textContent = 'Failed';
      setTimeout(() => {
        button.textContent = 'Copy';
      }, 2000);
    });
  }

  // Mark active nav link
  function setActiveNavLink() {
    const path = window.location.pathname;
    const links = document.querySelectorAll('.nav-links a');

    links.forEach(link => {
      link.classList.remove('active');
      const href = link.getAttribute('href');
      if (href === path || (href !== '/' && path.startsWith(href))) {
        link.classList.add('active');
      }
    });
  }

  // Initialize
  document.addEventListener('DOMContentLoaded', () => {
    // Copy buttons
    document.querySelectorAll('.copy-btn').forEach(btn => {
      btn.addEventListener('click', () => copyCode(btn));
    });

    // Active nav link
    setActiveNavLink();

    // Highlight.js initialization
    if (typeof hljs !== 'undefined') {
      hljs.highlightAll();
    }
  });

  // Expose for onclick handlers
  window.tsrunCopyCode = copyCode;
})();
