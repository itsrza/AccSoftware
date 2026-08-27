import React from 'react'

type Name = 'grid'|'receipt'|'cart'|'package'|'users'|'wallet'|'check'|'file'|'bar'|'settings'|'search'|'bell'|'plus'|'arrow'|'trend'|'bank'|'cash'|'warehouse'|'more'|'moon'|'sun'|'chevron'|'close'|'filter'|'download'|'refresh'|'box'|'factory'|'logout'

const paths: Record<Name, React.ReactNode> = {
  grid:<><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></>,
  receipt:<><path d="M6 3h12v18l-3-2-3 2-3-2-3 2z"/><path d="M9 8h6M9 12h6M9 16h4"/></>,
  cart:<><circle cx="9" cy="19" r="1.5"/><circle cx="18" cy="19" r="1.5"/><path d="M3 4h2l2.2 10.2a2 2 0 0 0 2 1.6h8.7a2 2 0 0 0 1.9-1.4L22 8H7"/></>,
  package:<><path d="m4 7 8-4 8 4-8 4-8-4Z"/><path d="M4 7v10l8 4 8-4V7M12 11v10"/></>,
  users:<><circle cx="9" cy="8" r="3"/><path d="M3 20c.7-3.3 2.8-5 6-5s5.3 1.7 6 5"/><path d="M16 5.5a3 3 0 0 1 0 5.8M17 15c2.7.3 4.1 1.8 4.7 4"/></>,
  wallet:<><path d="M4 6h15a2 2 0 0 1 2 2v11H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h13"/><path d="M2 8h17M16 13h3"/></>,
  check:<><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 9h6M7 13h5M16 13l1.2 1.2L20 11.5"/></>,
  file:<><path d="M6 3h9l4 4v14H6z"/><path d="M15 3v5h5M9 12h6M9 16h6"/></>,
  bar:<><path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/></>,
  settings:<><path d="M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4Z"/><path d="m19.4 15 .1.1 1.7 1.4-2 3-2-.9-.2.1-2.2.9-.4 2.2h-3.6l-.4-2.2-2.2-.9-.2-.1-2 .9-2-3L5.6 15l-.1-.2-.3-2.3L3.3 11l1.7-3.2 2.1.7.2-.1 1.9-1.1.4-2.1h3.6l.4 2.1 1.9 1.1.2.1 2.1-.7 1.7 3.2-1.9 1.5.1 2.3Z"/></>,
  search:<><circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/></>,
  bell:<><path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4"/></>,
  plus:<><path d="M12 5v14M5 12h14"/></>,
  arrow:<><path d="M5 12h14M13 6l6 6-6 6"/></>,
  trend:<><path d="M4 16 9 11l4 3 7-8"/><path d="M15 6h5v5"/></>,
  bank:<><path d="m3 9 9-5 9 5"/><path d="M5 10v7M9 10v7M15 10v7M19 10v7M3 20h18M2 9h20"/></>,
  cash:<><rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="12" cy="12" r="3"/><path d="M7 8h.01M17 16h.01"/></>,
  warehouse:<><path d="m3 10 9-6 9 6v10H3z"/><path d="M7 20v-6h10v6M7 10h10"/></>,
  more:<><circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/></>,
  moon:<path d="M20 15.2A8 8 0 0 1 8.8 4 8 8 0 1 0 20 15.2Z"/>,
  sun:<><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></>,
  chevron:<path d="m7 10 5 5 5-5"/>,
  close:<><path d="M6 6l12 12M18 6 6 18"/></>,
  filter:<><path d="M4 6h16M7 12h10M10 18h4"/></>,
  download:<><path d="M12 3v12M7 10l5 5 5-5M5 21h14"/></>,
  refresh:<><path d="M20 11a8 8 0 0 0-14.5-4L4 9"/><path d="M4 4v5h5M4 13a8 8 0 0 0 14.5 4L20 15"/><path d="M20 20v-5h-5"/></>,
  box:<><path d="m4 7 8-4 8 4v10l-8 4-8-4z"/><path d="m4 7 8 4 8-4M12 11v10"/></>,
  factory:<><path d="M3 21V9l7 4V9l7 4V5h4v16z"/><path d="M7 17h.01M11 17h.01M15 17h.01"/></>,
  logout:<><path d="M10 5H5v14h5"/><path d="M14 8l4 4-4 4M18 12H9"/></>
}

export function Icon({name,size=18}:{name:Name,size?:number}){
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>
}
