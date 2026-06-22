const logoVariant = (url, scheme) => {
  const target = scheme === "slate" ? "/dark/" : "/light/";
  return url.replace(/\/(?:light|dark)\//, target);
};

const updateThemeLogos = () => {
  const scheme = document.body.getAttribute("data-md-color-scheme");

  document.querySelectorAll(".md-logo img").forEach((logo) => {
    logo.dataset.logoSrc ||= logo.getAttribute("src") || "";
    logo.setAttribute("src", logoVariant(logo.dataset.logoSrc, scheme));
  });

  document.querySelectorAll('link[rel~="icon"]').forEach((icon) => {
    icon.dataset.logoHref ||= icon.getAttribute("href") || "";
    icon.setAttribute("href", logoVariant(icon.dataset.logoHref, scheme));
  });
};

updateThemeLogos();

new MutationObserver(updateThemeLogos).observe(document.body, {
  attributes: true,
  attributeFilter: ["data-md-color-scheme"],
});
