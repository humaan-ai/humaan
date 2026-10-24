const defaultLocation = "~/Humaan";

/** A bare name (or relative path) lives under the default project location. */
export function projectDestination(input: string) {
  const value = input.trim();
  const separator = value.includes("\\") ? "\\" : "/";
  const absolute = /^(?:\/|~[\\/]|[A-Za-z]:[\\/]|\\\\)/.test(value);
  const destination = absolute ? value : `${defaultLocation}/${value}`;
  const lastSeparator = Math.max(destination.lastIndexOf("/"), destination.lastIndexOf("\\"));
  const name = destination.slice(lastSeparator + 1);
  let location = destination.slice(0, lastSeparator);
  if (!location) location = separator;
  if (/^[A-Za-z]:$/.test(location)) location += separator;
  return { name, location, path: `${destination}${name ? "" : "<project name>"}` };
}
