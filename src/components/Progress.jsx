import './Progress.css'

function Progress({parts, total}){
    const sortedParts = parts ? [...parts].sort((a, b) => Number(a.start - b.start)) : [];
    return (
        <div className='Progress-Cover BG-Secondary'>  
            {sortedParts.map((part, index) => {
              const partSize = part.end - part.start + 1;
              if (partSize <= 0 || part.start >= 18446744073709551615) return null;
              const widthPct = total > 0 ? `${(partSize * 100) / total}%` : '100%';
              const fillPct = partSize > 0 ? `${(part.downloaded * 100) / partSize}%` : '0%';
              
              return (
                <div key={index} className='Progress-Div' style={{width: widthPct}}>
                    <div className='Progress-Fill BG-Quarternary' style={{width: fillPct}}></div>
                </div>
              );  
            })}
        </div>
    );
}

export default Progress;