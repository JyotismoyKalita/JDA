import './TitleBar.css'
import { getCurrentWindow } from "@tauri-apps/api/window";
import { VscChromeMaximize, VscChromeRestore } from 'react-icons/vsc';
import { MdClose, MdMinimize} from 'react-icons/md';
import { useState } from 'react';

function TitleBar(){
    const appWindow = getCurrentWindow();
    const [isMaximized, setIsMaximized] = useState(false);

    const toggleMaximize = async () => {
        const isMax = await appWindow.isMaximized();
        if (isMax) {
            await appWindow.unmaximize();
            setIsMaximized(false);
        } else {
            await appWindow.maximize();
            setIsMaximized(true);
        }
    };

    return (
        <div className='BG-Primary TitleBar Primary'>
            <div></div>
            <div className='Title'>JDA</div>
            <div className='Controls'>
                <div className='Controls-Cover' onClick={() => appWindow.minimize()}>           <MdMinimize /></div>
                <div className='Controls-Cover' onClick={() => toggleMaximize()}>
                    {isMaximized ? <VscChromeRestore /> : <VscChromeMaximize /> }
                </div>
                <div className='Controls-Cover' onClick={() => appWindow.close()}><MdClose /></div>
            </div>
        </div>
    );
}

export default TitleBar;