import './TabBar.css'
import '../theme/colors.css'
import Tab from './Tab';
import { useRef, useMemo } from 'react';

function TabBar({items, selectTab, data}){

    const ref = useRef(null);

    const counts = useMemo(() => {
        return data.reduce((acc, el) => {
            acc.All++;
            acc[el.state] = (acc[el.state] || 0) + 1;
            return acc;
        }, { All: 0 });
    }, [data]);



return (
    <div ref={ref} className='TabBar BG-Primary'>
        {items.map((item)=>(
            <div className='Tab-Cover' key={item.id} onClick={() => selectTab(item)}>
                <Tab 
                    id={item.id} 
                    caption={item.caption} 
                    isSelected={item.isSelected}
                    counts={counts}
                />
            </div>
        ))}
    </div>
);

}

export default TabBar;