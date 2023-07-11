////
////  ContentView.swift
////  iphone-app
////
////  Created by Könnecke, Alyssa on 20.06.23.
////


import SwiftUI
import Foundation
import Charts
import Starscream


enum Settings: String, CaseIterable{
    case ServerStuff, SettingsDebug, MessagesSent
}

struct DataEntry: Codable {
    let id = UUID()
    var timestamp: Date
    var measuredNumber: Float
    var isSent: Bool = false
    var isSaved: Bool = false
}

struct ServerEntry: Codable {
    let id = UUID()
    var timestamp: Date
    var measuredNumber: Float
}

class EntryGenerator: ObservableObject {
    @Published var lastReceivedMessage: [ServerEntry] = []{
        didSet {
            saveServerEntries()
        }
    }
    @Published var entries: [DataEntry] = []
    var timer: Timer?
    var timerRecon: Timer?
    @Published var wantToBeConnected : Bool = false
    @Published var useridUUID: String {
        didSet {
            UserDefaults.standard.set(useridUUID, forKey: "username")
        }
    }
    @Published var websocket: WebSocket?
    @Published var isServerConnected : Bool = false
    let url = URL(string: "http://localhost:3000/ws")!
   
    init() {
        self.useridUUID = UserDefaults.standard.object(forKey: "username") as? String ?? ""
        if (useridUUID.isEmpty){
            useridUUID = UUID().uuidString
        }
        if let data = UserDefaults.standard.data(forKey: "SavedData") {
            if let decoded = try? JSONDecoder().decode([DataEntry].self, from: data) {
                entries = decoded
                return
            }
        }
        if let dataServer = UserDefaults.standard.data(forKey: "SSData") {
            if let decoded = try? JSONDecoder().decode([ServerEntry].self, from: dataServer) {
                lastReceivedMessage = decoded
                return
            }
        }
    }
    
    func saveEntries() {
        if let encoded = try? JSONEncoder().encode(entries) {
            UserDefaults.standard.set(encoded, forKey: "SavedData")
        }
    }
    
    func saveServerEntries() {
        if let encoded = try? JSONEncoder().encode(lastReceivedMessage) {
            UserDefaults.standard.set(encoded, forKey: "SSData")
        }
    }
    
    func setupWebSocket() {
        if(websocket == nil){
            let request = URLRequest(url: url)
            //        let url = URL(string: "https://fog-cloud-zqzxhmscmq-ew.a.run.app/ws")!
            
            websocket = WebSocket(request: request)
            websocket?.delegate = self
        } else {
            connect()
        }
    }
    
    
    
    func generateEntry() {
        timer = Timer.scheduledTimer(withTimeInterval: 7, repeats: true) { _ in
            let newEntry = DataEntry(timestamp: NSDate.now, measuredNumber: Float.random(in: 0...1))
                    self.entries.append(newEntry)
                    self.saveEntries()
                    self.sendEntry(newEntry)
        }
    }

    private func sendEntry(_ entry: DataEntry) {
        guard isServerConnected && wantToBeConnected else {
            return
        }
        
        guard let websocket = websocket else {
            // WebSocket is not available
            return
        }
        
        let timestamp = "\(Int(entry.timestamp.timeIntervalSince1970))"
        let message = "SENSOR#" + useridUUID + "#" + timestamp + "#" + "\(entry.measuredNumber)"
        
        if(websocket.write(string: message) != nil) {
            if let index = self.entries.firstIndex(where: { $0.id == entry.id }) {
                     self.entries[index].isSent = true
            }
        }
    }

    
    
    func connect(){
        websocket?.connect()
        if (self.wantToBeConnected == true && self.isServerConnected == false){
                timerRecon = Timer.scheduledTimer(withTimeInterval: 8, repeats: true) { [weak self] _ in
                            guard let self = self else { return }
                            websocket?.connect()
                        }
        }
    }
    
     func disconnect(){
        self.isServerConnected = false
        timerRecon?.invalidate()
        websocket?.write(string: "DISCONN#" + "\(useridUUID)")
        websocket?.disconnect(closeCode: CloseCode.normal.rawValue)
    }
    
    private func sendEntriesWhichHaventBeenSend(){
        if (wantToBeConnected && isServerConnected){
            self.entries.forEach { entry in
                if(entry.isSent == false){
                    self.sendEntry(entry)
                }
            }
        }
    }
}

extension EntryGenerator: WebSocketDelegate {
    func didReceive(event: WebSocketEvent, client: WebSocket) {
        switch event {
        case .connected:
            print("WebSocket connected")
            let connecMessage = "CONN#" + useridUUID
            client.write(string: connecMessage)
            self.isServerConnected = true
            sendEntriesWhichHaventBeenSend()
            
        case .disconnected(let reason, _):
            print("WebSocket disconnected: \(reason)")
            self.isServerConnected = false

        case .text(let message):
            if(self.wantToBeConnected){
                print("Received message: \(message)")
                if let receivedEntry = parseReceivedEntry(from: message) {
                    self.lastReceivedMessage.append(receivedEntry)
                    self.saveServerEntries()
                }
            }
            self.isServerConnected = self.wantToBeConnected
            
            
        case .error(let error):
            self.isServerConnected = false
            print("WebSocket error: \(error)")
            handleError(error)
               
            
        case .cancelled:
            self.isServerConnected = false
            print("WebSocket cancelled ")
            
        default:
            break
        }
        
    }
    func handleError(_ error: Error?){
        if let e = error as? WSError {
            print("WebSocket encountered an error: \(e.message)")
           
            if e.message.contains("Connection reset by peer") {
                        if let lastIndex = entries.lastIndex(where: { $0.isSent }) {
                            let startIndex = min(lastIndex + 1, entries.count - 3)
                            let endIndex = entries.count - 1
                            for index in startIndex...endIndex {
                                entries[index].isSent = false
                            }
                        }
                    }

        } else if let e = error {
            print("WebSocket encountered an error: \(e.localizedDescription)")
           
            if e.localizedDescription.contains("Connection reset by peer") {
                      if let lastIndex = entries.lastIndex(where: { $0.isSent }) {
                          let startIndex = min(lastIndex + 1, entries.count - 3)
                          let endIndex = entries.count - 1
                          for index in startIndex...endIndex {
                              entries[index].isSent = false
                          }
                      }
                  }
            
        } else {
            print("WebSocket encountered an error")
        }
    }

    
    private func parseReceivedEntry(from message: String) -> ServerEntry? {
            var components = message.components(separatedBy: "#")
            components.removeFirst()
            guard components.count == 2,
                  let timestamp = Double(components[0]),
                  let measuredNumber = Float(components[1]) else {
                    print("whyyyyyyyyyyy")
                    return nil
            }
            let date = Date(timeIntervalSince1970: timestamp)
            return ServerEntry(timestamp: date, measuredNumber: measuredNumber)
        }
    }




struct ContentView: View {
    @ObservedObject var entryGenerator = EntryGenerator()
   

    var body: some View {
        TabView {
            NavigationView{
                List(entryGenerator.entries, id: \.timestamp){ dataEntry in
                    VStack{
                        Text("\(dataEntry.timestamp.ISO8601Format())")
                        Text("\(dataEntry.measuredNumber)").foregroundColor(dataEntry.isSent ? .green : .red) }
                }
                .navigationTitle(Text("Measurements"))
            }
            .tabItem {
                Image(systemName: "square.and.arrow.up.on.square")
                Text("Measurements")
            }
            
            .overlay(
                Circle()
                    .foregroundColor(entryGenerator.isServerConnected ? .green : .red)
                    .frame(width: 20, height: 20)
                    .padding(10)
                    .offset(x: UIScreen.main.bounds.width / 2 - 30, y: -UIScreen.main.bounds.height / 2 + 30)
                    .zIndex(1)
            )
            
            VStack{
                Button(action: {if (entryGenerator.wantToBeConnected == true) {
                    entryGenerator.wantToBeConnected = false
                    entryGenerator.disconnect()

                } else if (entryGenerator.wantToBeConnected == false) {
                    entryGenerator.wantToBeConnected = true
                    entryGenerator.connect()
                }
                }){
                    Text(entryGenerator.wantToBeConnected ? "Disconnect": "Connect")
                }
                .foregroundColor(entryGenerator.wantToBeConnected ? .red : .green)
                    .font(.system(size: 24))
                VStack{
                    Form {
                            Section(header: Text("PROFILE")) {
                                Text("Username\n" + entryGenerator.useridUUID)
                                }
                            }
                    Button("Delete Messages"){
                       entryGenerator.lastReceivedMessage = []
                        entryGenerator.entries = []
                        entryGenerator.saveServerEntries()
                        entryGenerator.saveEntries()
                    }
                }
            }
            .tabItem {
                Image(systemName: "gearshape")
                Text("Configuration")
            }
            VStack {
               if let lastReceivedMessage = entryGenerator.lastReceivedMessage.last {
                                    VStack {
                                        Text("Last received AVG")
                                            .font(.headline)
                                            .padding(.bottom, 4)

                                        Text("\(lastReceivedMessage.timestamp.ISO8601Format())")
                                        Text("\(lastReceivedMessage.measuredNumber)").foregroundColor(.blue)
                                    }
                                    .padding()
                }
                if !entryGenerator.lastReceivedMessage.isEmpty {
                        List(entryGenerator.lastReceivedMessage, id: \.timestamp){ dataEntry in
                            VStack{
                                    Text("\(dataEntry.timestamp.ISO8601Format())")
                                    Text("\(dataEntry.measuredNumber)")
                            }
                        }
                }
//                else {
//                                Text("No messages received")
//                                    .foregroundColor(.gray)
//                            }
            }.navigationTitle(Text("AVG"))
            .onAppear{
                entryGenerator.saveServerEntries()
            }
            .tabItem {
                Image(systemName: "square.and.arrow.down.on.square")
                Text("Server Messages")
            }
            
            }.onAppear {
                entryGenerator.setupWebSocket()
                entryGenerator.generateEntry()
        }
    }
    
    struct ContentView_Previews: PreviewProvider {
        static var previews: some View {
                ContentView()
        }
    }
}




