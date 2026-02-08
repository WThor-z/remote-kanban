export const shouldUseUiDemo = (pathname: string, search: string): boolean => {
  if (pathname === '/ui-demo' || pathname === '/ui-demo/') {
    return true;
  }

  const params = new URLSearchParams(search);
  return params.get('demo') === 'ui';
};
